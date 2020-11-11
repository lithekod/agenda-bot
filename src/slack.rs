use futures::join;
use slack::{
    Event,
    Message,
};
use tokio::{
    sync::mpsc,
    task::{
        spawn,
        spawn_blocking,
    },
};

struct Handler {
    sender: mpsc::UnboundedSender<String>,
}

impl Handler {
    fn new(sender: mpsc::UnboundedSender<String>) -> Self {
        Self {
            sender
        }
    }

    fn sender(&self) -> &mpsc::UnboundedSender<String> {
        &self.sender
    }
}

impl slack::EventHandler for Handler {
    fn on_event(&mut self, _cli: &slack::RtmClient, event: slack::Event) {
        println!("on_event: {:#?}", event);
        match event {
            Event::Message(msg) => {
                match *msg {
                    Message::Standard(msg) => {
                        self.sender().send(format!("{}:{} says: {}",
                                                   msg.channel.unwrap_or("??".to_string()),
                                                   msg.user.unwrap_or("??".to_string()),
                                                   msg.text.unwrap_or("??".to_string()))
                                           .to_string()).unwrap();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
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
    sender: mpsc::UnboundedSender<String>,
    mut receiver: mpsc::UnboundedReceiver<String>,
) {
    println!("Setting up Slack");

    let token = std::env::var("SLACK_API_TOKEN").unwrap_or(token.unwrap());
    let client = spawn_blocking(move || {
        slack::RtmClient::login(&token).unwrap()
    }).await.unwrap();

    let slack_sender = client.sender().clone();

    join!(
        spawn_blocking(move || {
            let mut handler = Handler::new(sender);
            match client.run(&mut handler) {
                Ok(_) => {}
                Err(e) => {
                    println!("Error: {}", e)
                }
            }
        }),
        spawn(async move {
            while let Some(s) = receiver.recv().await {
                println!("Slack received '{}'", s);
                //TODO Sending messages is very slow sometimes. Have seen delays
                // from 5 up to 20(!) seconds.
                slack_sender.send_typing("CPBAA5FA7").unwrap();
                println!("Typing");
                slack_sender.send_message("CPBAA5FA7", &s).unwrap();
                println!("Sent");
            }
        })
    );
}
