use crate::agenda::{
    parse_message,
    AgendaPoint
};

use discord::{
    model::{
        ChannelId,
        Event,
        PossibleServer,
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
const CHANNEL: Option<u64> = None;

pub async fn handle(
    sender: mpsc::UnboundedSender<AgendaPoint>,
    receiver: mpsc::UnboundedReceiver<AgendaPoint>,
) {
    println!("Setting up Discord");

    let token = std::env::var("DISCORD_API_TOKEN").unwrap_or_else(|_| TOKEN.expect("Missing Discord token").to_string());
    let client = Discord::from_bot_token(&token);

    if let Ok(client) = client {
        let (connection, _) = client.connect().expect("Discord connect failed"); //TODO
        let our_id = client.get_current_user().unwrap().id;
        let client = Arc::new(Mutex::new(client));

        let channel = match std::env::var("DISCORD_CHANNEL") {
            Ok(channel) => Some(ChannelId(channel.parse::<u64>().unwrap())),
            Err(_) => CHANNEL,
        };

        let (_, _) = join!( //TODO?
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
                if let PossibleServer::Online(server) = server {
                    println!("Discord channels in {}: {:#?}",
                             server.name,
                             server
                             .channels
                             .iter()
                             .map(|channel| format!("{}: {} ({:?})",
                                                    channel.name,
                                                    channel.id,
                                                    channel.kind))
                             .collect::<Vec<_>>());
                }
            }

            Ok(Event::MessageCreate(message)) => {
                if let Some(channel) = channel {
                    if let Ok(Some(s)) = parse_message(
                        &message.content,
                        &message.author.name,
                        &sender,
                    ) {
                        client.lock().unwrap().send_message(channel,
                                                            &s,
                                                            "",
                                                            false).unwrap();
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
            client.lock().unwrap().send_message(channel,
                                                &point.to_add_message(),
                                                "",
                                                false).unwrap();
        }
    }

}
