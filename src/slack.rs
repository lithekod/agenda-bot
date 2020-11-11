use futures::join;
use tokio::{
    sync::mpsc,
    task::{
        spawn,
        spawn_blocking,
    },
};

struct Handler;

impl slack::EventHandler for Handler {
    fn on_event(&mut self, _cli: &slack::RtmClient, event: slack::Event) {
        println!("on_event: {:#?}", event);
    }

    fn on_close(&mut self, _cli: &slack::RtmClient) {
        println!("on_close")
    }

    fn on_connect(&mut self, _cli: &slack::RtmClient) {
        println!("on_connect");
    }
}

pub async fn handle(
    token: Option<String>,
    _sender: mpsc::UnboundedSender<String>,
    mut receiver: mpsc::UnboundedReceiver<String>,
) {
    println!("Setting up Slack");

    let token = std::env::var("SLACK_API_TOKEN").unwrap_or(token.unwrap());

    join!(
        spawn_blocking(move || {
            let mut handler = Handler;
            match slack::RtmClient::login_and_run(&token, &mut handler) {
                Ok(_) => {}
                Err(e) => {
                    println!("Error: {}", e)
                }
            }
        }),
        spawn(async move {
            while let Some(s) = receiver.recv().await {
                println!("Slack received '{}' from discord", s);
            }
        })
    );
}
