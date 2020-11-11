use discord::{
    model::Event,
    Discord,
};
use slack_api as slack;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    println!("Setting up Slack");

    let slack_token = std::env::var("SLACK_API_TOKEN")
        .unwrap_or(""
                   .to_string());
    let slack_client = slack::default_client().unwrap();

    let slack_request = slack::rtm::StartRequest::default();
    let response = slack::rtm::start(&slack_client,
                                     &slack_token,
                                     &slack_request).await;

    if let Ok(response) = response {
        if let Some(channels) = response.channels {
            let channel_names = channels
                .iter()
                .filter_map(|c| c.name.as_ref())
                .collect::<Vec<_>>();
            println!("Got channels {:?}", channel_names);
        }

        if let Some(users) = response.users {
            let user_names = users
                .iter()
                .filter_map(|u| u.name.as_ref())
                .collect::<Vec<_>>();
            println!("Got users {:?}", user_names);
        }
    } else {
        println!("{:?}", response)
    }

    println!("Setting up Discord");

    let discord_token = std::env::var("DISCORD_API_TOKEN")
        .unwrap_or(""
                   .to_string());
    let discord = Discord::from_bot_token(&discord_token);

    if let Ok(discord) = discord {
        let (mut connection, _) = discord.connect() .expect("discord connect failed");
        println!("Discord ready");
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
    }
}
