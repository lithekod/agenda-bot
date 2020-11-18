use crate::agenda::{
    parse_message,
    AgendaPoint,
    Emoji,
};

use discord::{
    model::{
        ChannelId,
        Event,
        PossibleServer,
        ReactionEmoji,
    },
    Discord,
    Error,
};
use futures::join;
use std::sync::{
    Arc,
    Mutex,
};
use tokio::{
    sync::mpsc,
    task::{
        spawn,
        spawn_blocking,
    },
};

const TOKEN: Option<&str> = None;
const CHANNEL: Option<ChannelId> = None;

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
        let our_id = client.get_current_user().unwrap().id;
        let client = Arc::new(Mutex::new(client));

        let channel = std::env::var("DISCORD_CHANNEL")
            .map(|id| Some(ChannelId(id.parse::<u64>().unwrap())))
            .unwrap_or(CHANNEL);

        let (_, _) = join!(
            spawn(receive_from_slack(receiver, Arc::clone(&client), channel)),
            spawn_blocking(move || receive_events(our_id, connection, sender, client, channel)),
        );
    }
}

fn receive_events(
    _our_id: discord::model::UserId,
    mut connection: discord::Connection,
    sender: mpsc::UnboundedSender<AgendaPoint>,
    client: Arc<Mutex<discord::Discord>>,
    channel: Option<ChannelId>,
) {
    loop {
        match connection.recv_event() {
            Ok(Event::ServerCreate(server)) => {
                match (channel, server) {
                    (None, PossibleServer::Online(server)) => {
                        println!("Discord channels in {}: {:#?}",
                                 server.name,
                                 server
                                 .channels
                                 .iter()
                                 .map(|channel|
                                      format!("{}: {} ({:?})",
                                              channel.name,
                                              channel.id,
                                              channel.kind))
                                 .collect::<Vec<_>>());
                    }
                    (None, PossibleServer::Offline(server)) => {
                        println!("Server {} is offline", server);
                    }
                    (Some(_), _) => {}
                }
            }

            Ok(Event::MessageCreate(message)) => {
                if let Some(channel) = channel {
                    if channel == message.channel_id {
                        match parse_message(
                            &message.content,
                            &message.author.name,
                            |s: String| {
                                client
                                    .lock()
                                    .unwrap()
                                    .send_message(channel, &s, "", false)
                                    .unwrap();
                            },
                            &sender
                        ) {
                            Some(Emoji::Ok) => {
                                client.lock().unwrap().add_reaction(
                                    channel,
                                    message.id,
                                    ReactionEmoji::Unicode("👍".to_string())
                                ).unwrap();
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
    channel: Option<ChannelId>
) {
    if let Some(channel) = channel {
        while let Some(point) = receiver.recv().await {
            println!("Discord received '{}'", point);
            client.lock().unwrap().send_message(
                channel,
                &point.to_add_message(),
                "",
                false
            ).unwrap();
        }
    }

}
