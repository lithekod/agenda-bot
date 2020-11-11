use discord::{
    model::{
        ChannelId,
        Event,
    },
    Discord,
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
    sender: mpsc::UnboundedSender<String>,
    mut receiver: mpsc::UnboundedReceiver<String>,
) {
    println!("Setting up Discord");

    let token = std::env::var("DISCORD_API_TOKEN").unwrap_or(token.unwrap());
    let client = Discord::from_bot_token(&token);

    if let Ok(client) = client {
        let (mut connection, _) = client.connect().expect("discord connect failed"); //TODO
        let our_id = client.get_current_user().unwrap().id;
        println!("Discord ready");

        let (_, _) = join!( //TODO?
            spawn_blocking(move || {
                loop {
                    match connection.recv_event() {
                        Ok(Event::MessageCreate(message)) => {
                            if message.author.id != our_id {
                                sender.send(format!("{:?}:{} says: {}",
                                                    message.channel_id,
                                                    message.author.name,
                                                    message.content))
                                    .unwrap();
                            }
                        }
                        Ok(_) => {}
                        Err(discord::Error::Closed(code, body)) => {
                            println!("Discord closed with code {:?}: {}", code, body);
                            break;
                        }
                        Err(err) => {
                            println!("Error: {:?}", err);
                        }
                    }
                }
            }),
            spawn(async move {
                while let Some(s) = receiver.recv().await {
                    println!("Discord received '{}' from slack", s);
                    client.send_message(ChannelId(697057150106599488), //TODO
                                        &s,
                                        "",
                                        false
                    );
                }
            })
        );
    }
}
