use crate::agenda::{
    parse_message,
    AgendaPoint
};

use discord::{
    model::{
        ChannelId,
        Event,
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
    let channel_id = ChannelId(
        match std::env::var("DISCORD_CHANNEL") {
            Ok(var) => var.parse().unwrap(),
            Err(_) => CHANNEL.expect("Missing Discord channel"),
        });

    let client = Discord::from_bot_token(&token);

    if let Ok(client) = client {
        let (connection, _) = client.connect().expect("Discord connect failed"); //TODO
        let our_id = client.get_current_user().unwrap().id;
        let client = Arc::new(Mutex::new(client));
        println!("Discord ready");

        let (_, _) = join!( //TODO?
            spawn(receive_from_slack(receiver, Arc::clone(&client), channel_id)),
            spawn_blocking(move || receive_events(our_id, connection, sender, client, channel_id)),
        );
    }
}

fn receive_events(
    our_id: discord::model::UserId,
    mut connection: discord::Connection,
    sender: mpsc::UnboundedSender<AgendaPoint>,
    client: Arc<Mutex<discord::Discord>>,
    channel_id: ChannelId,
) {
    loop {
        match connection.recv_event() {
            Ok(Event::MessageCreate(message)) => {
                if let Ok(Some(s)) = parse_message(&message.content, &message.author.name) {
                    client.lock().unwrap().send_message(channel_id,
                                                        &s,
                                                        "",
                                                        false).unwrap();
                }
                //if message.author.id != our_id {
                //    sender.send(AgendaPoint{
                //        title: message.content,
                //        adder: message.author.name,
                //    }).unwrap();
                //}
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
    channel_id: ChannelId
) {
    while let Some(point) = receiver.recv().await {
        println!("Discord received '{}'", point);
        client.lock().unwrap().send_message(channel_id,
                                            &point.to_add_message(),
                                            "",
                                            false).unwrap();
    }

}
