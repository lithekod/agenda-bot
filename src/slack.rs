use crate::agenda::{
    parse_message,
    AgendaPoint
};

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

const TOKEN: Option<&str> = None;
const CHANNEL: Option<&str> = None;

struct Handler {
    sender: mpsc::UnboundedSender<AgendaPoint>,
    slack_sender: slack::Sender,
    slack_channel: String,
}

impl Handler {
    fn new(
        sender: mpsc::UnboundedSender<AgendaPoint>,
        slack_sender: slack::Sender,
        slack_channel: String
    ) -> Self {
        Self {
            sender,
            slack_sender,
            slack_channel,
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
                        if let Ok(Some(s)) = parse_message(
                            &msg.text.unwrap_or("".to_string()),
                            &msg.user.unwrap_or("??".to_string()),
                        ) {
                            self.slack_sender.send_message(self.slack_channel.as_str(), &s).unwrap();
                        }
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
    sender: mpsc::UnboundedSender<AgendaPoint>,
    receiver: mpsc::UnboundedReceiver<AgendaPoint>,
) {
    println!("Setting up Slack");

    let token = std::env::var("SLACK_API_TOKEN").unwrap_or_else(|_| TOKEN.expect("Missing slack token").to_string());
    let token_clone = token.clone();
    let client = spawn_blocking(move || {
        slack::RtmClient::login(&token).unwrap()
    }).await.unwrap();

    let mut handler = Handler::new(sender, client.sender().clone(), token_clone);
    let slack_sender = client.sender().clone();

    let (_, _) = join!(
        spawn_blocking(move || {
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
        //TODO Sending messages is very slow sometimes. Have seen delays
        // from 5 up to 20(!) seconds.
        let channel = std::env::var("SLACK_CHANNEL").unwrap_or_else(|_| CHANNEL.expect("Missing slack channel").to_string());
        sender.send_typing(&channel).unwrap();
        sender.send_message(&channel, &point.to_add_message()).unwrap();
        println!("Slack message sent");
    }
}
