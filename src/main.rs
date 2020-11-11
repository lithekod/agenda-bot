use discord::{
    model::Event,
    Discord,
};
use futures::join;
use slack_api as slack;
use tokio::{
    sync::mpsc,
    task::{
        spawn,
        spawn_blocking,
    },
};

#[tokio::main]
async fn main() {
    let slack_token: Option<String> = None;
    let discord_token: Option<String> = None;

    println!("Hello, world!");

    let (from_discord, to_slack) = mpsc::unbounded_channel::<String>();
    let (from_slack, to_discord) = mpsc::unbounded_channel::<String>();

    join!(
        discord_loop(discord_token, from_discord, to_discord),
        slack_loop(slack_token, from_slack, to_slack),
    );
}

async fn slack_loop(
    token: Option<String>,
    sender: mpsc::UnboundedSender<String>,
    _receiver: mpsc::UnboundedReceiver<String>,
) {
    println!("Setting up Slack");

    let token = std::env::var("SLACK_API_TOKEN")
        .unwrap_or(token.unwrap());
    let client = slack::default_client().unwrap();

    let request = slack::rtm::StartRequest::default();
    let response = slack::rtm::start(&client,
                                     &token,
                                     &request).await;

    if let Ok(response) = response {
        if let Some(channels) = response.channels {
            let channel_names = channels
                .iter()
                .filter_map(|c| c.name.as_ref())
                .collect::<Vec<_>>();
            sender.send(format!("Got channels {:?}", channel_names).to_string()).unwrap();
        }

        if let Some(users) = response.users {
            let user_names = users
                .iter()
                .filter_map(|u| u.name.as_ref())
                .collect::<Vec<_>>();
            sender.send(format!("Got users {:?}", user_names).to_string()).unwrap();
        }
    } else { //TODO NotAuth etc
        println!("{:?}", response)
    }
}

async fn discord_loop(
    token: Option<String>,
    _sender: mpsc::UnboundedSender<String>,
    mut receiver: mpsc::UnboundedReceiver<String>,
) {
    println!("Setting up Discord");

    let token = std::env::var("DISCORD_API_TOKEN")
        .unwrap_or(token.unwrap());
    let client = Discord::from_bot_token(&token);

    if let Ok(client) = client {
        let (mut connection, _) = client.connect().expect("discord connect failed"); //TODO
        println!("Discord ready");

        let (_, _) = join!( //TODO
            spawn_blocking(move || {
                loop {
                    match connection.recv_event() {
                        Ok(Event::MessageCreate(message)) => {
                            println!("{} says: {}", message.author.name, message.content);
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
                }
            })
        );
    }
}
