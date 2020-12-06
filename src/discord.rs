use crate::{Service, To, is_to_me, kodapa};

use discord::{
    model::{ChannelId, Event, PossibleServer, ReactionEmoji, UserId},
    Discord, Error,
};
use futures::join;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{runtime::Runtime, sync::{mpsc, oneshot}, task::{spawn_blocking}};

const TOKEN: Option<&str> = None;
const CHANNEL: Option<ChannelId> = None;

#[derive(Clone)]
struct Handler {
    _our_id: UserId,
    client: Arc<Mutex<discord::Discord>>,
    channel: Option<ChannelId>,
    display_names: Arc<Mutex<HashMap<UserId, String>>>,
}

impl Handler {
    fn send_message(&self, message: &str) {
        self.client.lock().unwrap().send_message(self.channel.unwrap(), message, "", false).unwrap();
    }
}

pub async fn handle(
    request_sender: mpsc::UnboundedSender<kodapa::Request>,
    event_receiver: mpsc::UnboundedReceiver<kodapa::Event>,
) {
    println!("Setting up Discord");

    let token = std::env::var("DISCORD_API_TOKEN")
        .unwrap_or_else(|_| TOKEN.expect("Missing Discord token").to_string());
    let client = Discord::from_bot_token(&token);

    if let Ok(client) = client {
        let (connection, _) = client.connect().expect("Discord connect failed"); //TODO
        let _our_id = client.get_current_user().unwrap().id;
        let client = Arc::new(Mutex::new(client));
        let display_names = Arc::new(Mutex::new(HashMap::new()));

        let channel = std::env::var("DISCORD_CHANNEL")
            .map(|id| Some(ChannelId(id.parse::<u64>().unwrap())))
            .unwrap_or(CHANNEL);

        let handler = Handler {
            _our_id,
            client: client.clone(),
            channel,
            display_names,
        };

        let _ = join!(
            receive_kodapa_events(event_receiver, handler.clone()),
            spawn_blocking(|| receive_discord_events(handler, connection, request_sender)),
        );
    }
}

async fn receive_kodapa_events(
    mut event_receiver: mpsc::UnboundedReceiver<kodapa::Event>,
    handler: Handler
) {
    loop {
        let event = event_receiver.recv().await.unwrap();
        if is_to_me!(event.to, Service::Discord) {
            handler.send_message(&event.message);
        }
    }
}

fn receive_discord_events(
    handler: Handler,
    mut connection: discord::Connection,
    request_sender: mpsc::UnboundedSender<kodapa::Request>
) {
    loop {
        match connection.recv_event() {
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
                            handler.display_names.lock().unwrap().insert(member.user.id, nick);
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
                        let (feedback_sender, feedback_receiver) = oneshot::channel::<kodapa::Feedback>();
                        request_sender.send(kodapa::Request{
                            origin: Service::Discord,
                            message: message.content,
                            sender: if let Some(display_name) =
                                handler.display_names.lock().unwrap().get(&message.author.id)
                            {
                                display_name.to_string()
                            } else {
                                println!("Missing display name for '{}' (see 'Discord display names' in the readme)",
                                         message.author.name);
                                message.author.name
                            },
                            feedback: Some(feedback_sender),
                        }).unwrap();

                        let feedback = Runtime::new().unwrap().block_on(feedback_receiver);
                        match feedback {
                            Ok(kodapa::Feedback::Ok) => {
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
                //TODO restart
                break;
            }
            Err(e) => {
                println!("Discord error: {:?}", e);
            }
        }
    }
}
