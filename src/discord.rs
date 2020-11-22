use crate::agenda::{self, parse_message, AgendaPoint, Emoji};

use discord::{
    model::{ChannelId, Event, PossibleServer, ReactionEmoji, UserId},
    Discord, Error,
};
use futures::join;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::mpsc,
    task::{spawn, spawn_blocking},
};

const TOKEN: Option<&str> = None;
const CHANNEL: Option<ChannelId> = None;

struct Handler {
    _our_id: UserId,
    connection: discord::Connection,
    sender: mpsc::UnboundedSender<AgendaPoint>,
    client: Arc<Mutex<discord::Discord>>,
    channel: Option<ChannelId>,
    display_names: HashMap<UserId, String>,
}

pub async fn handle(
    sender: mpsc::UnboundedSender<AgendaPoint>,
    receiver: mpsc::UnboundedReceiver<AgendaPoint>,
) {
    println!("Setting up Discord");

    let token = std::env::var("DISCORD_API_TOKEN")
        .unwrap_or_else(|_| TOKEN.expect("Missing Discord token").to_string());
    let client = Discord::from_bot_token(&token);

    if let Ok(client) = client {
        let (connection, _) = client.connect().expect("Discord connect failed"); //TODO
        let _our_id = client.get_current_user().unwrap().id;
        let client = Arc::new(Mutex::new(client));

        let channel = std::env::var("DISCORD_CHANNEL")
            .map(|id| Some(ChannelId(id.parse::<u64>().unwrap())))
            .unwrap_or(CHANNEL);

        let (_, _) = join!(
            spawn(receive_from_slack(receiver, Arc::clone(&client), channel)),
            spawn_blocking(move || receive_events(&mut Handler {
                _our_id,
                connection,
                sender,
                client,
                channel,
                display_names: HashMap::new(),
            })),
        );
    }
}

fn receive_events(handler: &mut Handler) {
    loop {
        match handler.connection.recv_event() {
            Ok(Event::ServerCreate(server)) => {
                if let PossibleServer::Online(server) = server {
                    if handler.channel.is_none() {
                        println!(
                            "Discord channels in {}: {:#?}",
                            server.name,
                            server
                                .channels
                                .iter()
                                .map(|channel| format!(
                                    "{}: {} ({:?})",
                                    channel.name, channel.id, channel.kind
                                ))
                                .collect::<Vec<_>>()
                        );
                    }
                    for member in server.members {
                        if let Some(nick) = member.nick {
                            handler.display_names.insert(member.user.id, nick);
                        }
                    }
                } else if let PossibleServer::Offline(server) = server {
                    if handler.channel.is_none() {
                        println!("Server {} is offline", server);
                    }
                }
            }

            Ok(Event::MessageCreate(message)) => {
                if let Some(channel) = handler.channel {
                    if channel == message.channel_id {
                        match parse_message(
                            &message.content,
                            if let Some(display_name) =
                                handler.display_names.get(&message.author.id)
                            {
                                display_name
                            } else {
                                println!("Missing display name for '{}' (see 'Discord display names' in the readme)",
                                         message.author.name);
                                &message.author.name
                            },
                            |s: String| {
                                handler
                                    .client
                                    .lock()
                                    .unwrap()
                                    .send_message(channel, &s, "", false)
                                    .unwrap();
                            },
                            &handler.sender,
                        ) {
                            Some(Emoji::Ok) => {
                                handler
                                    .client
                                    .lock()
                                    .unwrap()
                                    .add_reaction(
                                        channel,
                                        message.id,
                                        ReactionEmoji::Unicode("👍".to_string()),
                                    )
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(Error::Closed(code, body)) => {
                println!("Discord closed with code {:?}: {}", code, body);
                break;
            }
            Err(e) => {
                println!("Discord error: {:?}", e);
            }
        }
    }
}

async fn receive_from_slack(
    mut receiver: mpsc::UnboundedReceiver<AgendaPoint>,
    client: Arc<Mutex<discord::Discord>>,
    channel: Option<ChannelId>,
) {
    if let Some(channel) = channel {
        while let Some(point) = receiver.recv().await {
            println!("Discord received '{}'", point);
            client
                .lock()
                .unwrap()
                .send_message(channel, &point.to_add_message(), "", false)
                .unwrap();
        }
    }
}
