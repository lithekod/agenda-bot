use crate::agenda::AgendaPoint;

use discord::{
    model::{
        ChannelId,
        Event,
    },
    Discord,
    Error,
};
use futures::join;
use tokio::{
    sync::mpsc,
    task::{
        spawn,
        spawn_blocking,
    },
};

pub async fn handle(
    token: Option<String>,
    sender: mpsc::UnboundedSender<AgendaPoint>,
    receiver: mpsc::UnboundedReceiver<AgendaPoint>,
) {
    println!("Setting up Discord");

    let token = std::env::var("DISCORD_API_TOKEN").unwrap_or(token.unwrap());
    let client = Discord::from_bot_token(&token);

    if let Ok(client) = client {
        let (connection, _) = client.connect().expect("Discord connect failed"); //TODO
        let our_id = client.get_current_user().unwrap().id;
        println!("Discord ready");

        let (_, _) = join!( //TODO?
            spawn_blocking(move || receive_events(our_id, connection, sender)),
            spawn(receive_from_slack(receiver, client))
        );
    }
}

fn receive_events(
    our_id: discord::model::UserId,
    mut connection: discord::Connection,
    sender: mpsc::UnboundedSender<AgendaPoint>
) {
    loop {
        match connection.recv_event() {
            Ok(Event::MessageCreate(message)) => {
                if message.author.id != our_id {
                    sender.send(AgendaPoint{
                        title: message.content,
                        adder: message.author.name,
                    }).unwrap();
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
    client: discord::Discord,
) {
    while let Some(point) = receiver.recv().await {
        println!("Discord received '{}'", point);
        client.send_message(ChannelId(697057150106599488), //TODO
                            &point.to_add_message(),
                            "",
                            false
        );
    }

}
