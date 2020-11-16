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
    slack_channel: Option<String>,
    print_channels: bool,
}

impl Handler {
    fn new(
        sender: mpsc::UnboundedSender<AgendaPoint>,
        slack_sender: slack::Sender,
        slack_channel: Option<String>,
    ) -> Self {
        Self {
            sender,
            slack_sender,
            slack_channel: slack_channel.clone(),
            print_channels: slack_channel.is_none()
        }
    }
}

impl slack::EventHandler for Handler {
    fn on_event(&mut self, cli: &slack::RtmClient, event: slack::Event) {
        match event {
            Event::Hello => {
                if self.print_channels {
                    println!("Slack channels found: {:#?}",
                             cli
                             .start_response()
                             .channels
                             .as_ref()
                             .and_then(|channels| {
                                 Some(channels
                                      .iter()
                                      .map(|channel| format!("{}: {}",
                                                             channel.name.as_ref().unwrap_or(&"??".to_string()), //TODO &"".to_string() ?
                                                             channel.id.as_ref().unwrap_or(&"??".to_string())))  //TODO
                                      .collect::<Vec<_>>())
                             }));
                }
            }
            Event::Message(msg) => {
                if let Some(channel) = &self.slack_channel {
                    match *msg {
                        Message::Standard(msg) => {
                            if msg.channel.is_some() && *channel == msg.channel.unwrap() { //TODO
                                if let Ok(Some(s)) = parse_message(
                                    &msg.text.unwrap_or("".to_string()),
                                    &msg.user.unwrap_or("??".to_string()),
                                    &self.sender,
                                ) {
                                    self.slack_sender.send_message(channel.as_str(), &s).unwrap();
                                }
                            }
                        }
                        _ => {} // message type
                    }
                }
            }
            _ => {} // event type
        }
    }

    fn on_close(&mut self, _cli: &slack::RtmClient) {}

    fn on_connect(&mut self, _cli: &slack::RtmClient) {}
}

pub async fn handle(
    sender: mpsc::UnboundedSender<AgendaPoint>,
    receiver: mpsc::UnboundedReceiver<AgendaPoint>,
) {
    println!("Setting up Slack");

    let token = std::env::var("SLACK_API_TOKEN").unwrap_or_else(|_| TOKEN.expect("Missing slack token").to_string());
    let channel = match std::env::var("SLACK_CHANNEL") {
        Ok(channel) => Some(channel),
        Err(_) => match CHANNEL {
            Some(channel) => Some(channel.to_string()),
            None => None
        }
    };
    let client = spawn_blocking(move || {
        slack::RtmClient::login(&token).unwrap()
    }).await.unwrap();

    let mut handler = Handler::new(sender, client.sender().clone(), channel.clone());
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
        spawn(receive_from_discord(receiver, slack_sender, channel))
    );
}

async fn receive_from_discord(
    mut receiver: mpsc::UnboundedReceiver<AgendaPoint>,
    sender: slack::Sender,
    channel: Option<String>,
) {
    if let Some(channel) = channel {
        while let Some(point) = receiver.recv().await {
            //TODO Sending messages is very slow sometimes. Have seen delays
            // from 5 up to 20(!) seconds.
            sender.send_typing(&channel).unwrap();
            sender.send_message(&channel, &point.to_add_message()).unwrap();
            println!("Slack message sent");
        }
    }
}
