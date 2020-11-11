use slack_api as slack;
use tokio::sync::mpsc;

pub async fn handle(
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
