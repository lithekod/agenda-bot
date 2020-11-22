use crate::agenda::{self, parse_message, AgendaPoint, Emoji};
use crate::reminder::ReminderType;

use futures::join;
use slack::{Event, Message};
use slack_api::{reactions, users};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex},
};
use tokio::{
    runtime::Runtime,
    sync::{mpsc, watch},
    task::{spawn, spawn_blocking},
};
use tokio_compat_02::FutureExt;

const TOKEN: Option<&str> = None;
const CHANNEL: Option<&str> = None;

struct Handler {
    sender: mpsc::UnboundedSender<AgendaPoint>,
    slack_sender: slack::Sender,
    slack_channel: Option<String>,
    print_channels: bool,
    slack_token: String,
    display_names: Arc<Mutex<HashMap<String, String>>>,
}

impl Handler {
    fn new(
        sender: mpsc::UnboundedSender<AgendaPoint>,
        slack_sender: slack::Sender,
        slack_channel: Option<String>,
        slack_token: String,
    ) -> Self {
        Self {
            sender,
            slack_sender,
            slack_channel: slack_channel.clone(),
            print_channels: slack_channel.is_none(),
            slack_token,
            display_names: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

async fn get_or_insert_display_name(
    display_names: Arc<Mutex<HashMap<String, String>>>,
    user_id: String,
    slack_token: &str,
) -> String {
    match display_names.lock().unwrap().entry(user_id.clone()) {
        Entry::Occupied(o) => o.get().to_string(),
        Entry::Vacant(v) => {
            let client = slack_api::requests::default_client().unwrap();
            if let Some(user) =
                users::list(&client, slack_token, &users::ListRequest { presence: None })
                    .compat()
                    .await
                    .unwrap()
                    .members
                    .unwrap()
                    .iter()
                    .find(|user| user.id.is_some() && user.id.as_deref().unwrap() == user_id)
            {
                v.insert(user.real_name.as_ref().unwrap().clone())
                    .to_string()
            } else {
                user_id
            }
        }
    }
}

impl slack::EventHandler for Handler {
    fn on_event(&mut self, cli: &slack::RtmClient, event: slack::Event) {
        match event {
            Event::Hello => {
                if self.print_channels {
                    println!(
                        "Slack channels found: {:#?}",
                        cli.start_response().channels.as_ref().map(|channels| {
                            channels
                                .iter()
                                .map(|channel| {
                                    format!(
                                        "{}: {}",
                                        channel.name.as_deref().unwrap_or("??"),
                                        channel.id.as_deref().unwrap_or("??"),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                    );
                }
            }
            Event::Message(msg) => {
                if let Some(channel) = &self.slack_channel.clone() {
                    match *msg {
                        Message::Standard(msg) => {
                            if msg.channel.is_some() && *channel == msg.channel.unwrap() {
                                //TODO
                                let user = match msg.user {
                                    Some(s) => Runtime::new().unwrap().block_on(
                                        get_or_insert_display_name(
                                            Arc::clone(&self.display_names),
                                            s,
                                            &self.slack_token,
                                        )
                                        .compat(),
                                    ),
                                    None => "??".to_string(),
                                };
                                match parse_message(
                                    &msg.text.unwrap_or("".to_string()),
                                    &user,
                                    |s: String| {
                                        self.slack_sender
                                            .send_message(channel.as_str(), &s)
                                            .unwrap();
                                    },
                                    &self.sender,
                                ) {
                                    Some(Emoji::Ok) => {
                                        let client = slack_api::requests::default_client().unwrap();
                                        Runtime::new()
                                            .unwrap()
                                            .block_on(
                                                reactions::add(
                                                    &client,
                                                    &self.slack_token,
                                                    &reactions::AddRequest {
                                                        name: "+1",
                                                        file: None,
                                                        file_comment: None,
                                                        channel: Some(channel.as_str()),
                                                        timestamp: Some(msg.ts.unwrap()),
                                                    },
                                                )
                                                .compat(),
                                            )
                                            .unwrap();
                                    }
                                    _ => {} // parse_message return
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
    reminder: watch::Receiver<ReminderType>,
) {
    println!("Setting up Slack");

    let token = std::env::var("SLACK_API_TOKEN")
        .unwrap_or_else(|_| TOKEN.expect("Missing slack token").to_string());
    let channel = match std::env::var("SLACK_CHANNEL") {
        Ok(channel) => Some(channel),
        Err(_) => match CHANNEL {
            Some(channel) => Some(channel.to_string()),
            None => None,
        },
    };
    let slack_token = token.to_string();
    let client = spawn_blocking(move || slack::RtmClient::login(&token).unwrap())
        .await
        .unwrap();

    let mut handler = Handler::new(
        sender,
        client.sender().clone(),
        channel.clone(),
        slack_token,
    );
    let slack_sender = client.sender().clone();

    let (_, _, _) = join!(
        spawn(receive_from_discord(
            receiver,
            slack_sender.clone(),
            channel.clone()
        )),
        spawn(handle_reminders(reminder, slack_sender, channel)),
        spawn_blocking(move || {
            match client.run(&mut handler) {
                Ok(_) => {}
                Err(e) => println!("Error: {}", e),
            }
        }),
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
            sender
                .send_message(&channel, &point.to_add_message())
                .unwrap();
            println!("Slack message sent");
        }
    }
}

async fn handle_reminders(
    mut reminder: watch::Receiver<ReminderType>,
    sender: slack::Sender,
    channel: Option<String>,
) {
    if let Some(channel) = channel {
        while let Some(reminder) = reminder.recv().await {
            match reminder {
                ReminderType::OneHour => {
                    sender.send_typing(&channel).unwrap();
                    sender
                        .send_message(
                            &channel,
                            &format!("Meeting in one hour!\n{}", agenda::read_agenda()),
                        )
                        .unwrap();
                }
                ReminderType::Void => {}
            }
        }
    }
}
