use crate::agenda::AgendaPoint;

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
    sender: mpsc::UnboundedSender<AgendaPoint>,
}

impl Handler {
    fn new(sender: mpsc::UnboundedSender<AgendaPoint>) -> Self {
        Self {
            sender
        }
    }

    fn sender(&self) -> &mpsc::UnboundedSender<AgendaPoint> {
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
                        self.sender().send(AgendaPoint{
                            title: msg.text.unwrap_or("??".to_string()),
                            adder: msg.user.unwrap_or("??".to_string()),
                        });
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
    sender: mpsc::UnboundedSender<AgendaPoint>,
    receiver: mpsc::UnboundedReceiver<AgendaPoint>,
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
        spawn(receive_from_discord(receiver, slack_sender))
    );
}

async fn receive_from_discord(
    mut receiver: mpsc::UnboundedReceiver<AgendaPoint>,
    sender: slack::Sender,
) {
    while let Some(point) = receiver.recv().await {
        println!("Slack received '{}'", point);
        //TODO Sending messages is very slow sometimes. Have seen delays
        // from 5 up to 20(!) seconds.
        sender.send_typing("CPBAA5FA7").unwrap();
        println!("Typing");
        sender.send_message("CPBAA5FA7", &point.to_add_message()).unwrap();
        println!("Sent");
    }
}
