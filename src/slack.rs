use crate::{Service, To, is_to_me, kodapa};

use slack::{error::Error, Event, Message};
use slack_api::{reactions, users};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex},
};
use tokio::{runtime::Runtime, sync::{mpsc, oneshot, watch}, task::{spawn, spawn_blocking}, try_join};
use tokio_compat_02::FutureExt;
const TOKEN: Option<&str> = None;
const CHANNEL: Option<&str> = None;


struct Handler {
    request_sender: mpsc::UnboundedSender<kodapa::Request>,
    slack_sender: slack::Sender,
    slack_channel: Option<String>,
    print_channels: bool,
    slack_token: String,
    display_names: Arc<Mutex<HashMap<String, String>>>,
}

impl Handler {
    fn new(
        request_sender: mpsc::UnboundedSender<kodapa::Request>,
        slack_sender: slack::Sender,
        slack_channel: Option<String>,
        slack_token: String,
    ) -> Self {
        Self {
            request_sender,
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

                                let (feedback_sender, feedback_receiver) = oneshot::channel::<kodapa::Feedback>();
                                self.request_sender.send(kodapa::Request{
                                    origin: Service::Slack,
                                    message: msg.text.unwrap_or_else(|| "???".to_string()),
                                    sender: user,
                                    feedback: Some(feedback_sender),
                                }).unwrap();

                                let feedback = Runtime::new().unwrap().block_on(feedback_receiver);
                                match feedback {
                                    Ok(kodapa::Feedback::Ok) => {
                                        let client = slack_api::requests::default_client().unwrap(); //TODO save client
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
    request_sender: mpsc::UnboundedSender<kodapa::Request>,
    mut event_receiver: mpsc::UnboundedReceiver<kodapa::Event>,
) -> ! {
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
    loop {
        let token_clone = token.clone();
        let client = spawn_blocking(move || slack::RtmClient::login(&token_clone).unwrap())
            .await
            .unwrap();

        let mut handler = Handler::new(
            request_sender.clone(),
            client.sender().clone(),
            channel.clone(),
            token.clone(),
        );
        let _ = try_join!(
            receive_kodapa_events(&mut event_receiver, client.sender().clone(), channel.clone()),
            spawn_blocking(move || {
                match client.run(&mut handler) {
                    Ok(_) => {}
                    Err(Error::WebSocket(_)) => {
                        println!("Restarting slack");
                        return Err(());
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        return Err(());
                    }
                }
                Ok(())
            }),
        );
    }
}

async fn receive_kodapa_events(
    event_receiver: &mut mpsc::UnboundedReceiver<kodapa::Event>,
    sender: slack::Sender,
    channel: Option<String>,
) -> Result<(), tokio::task::JoinError> {
    loop {
        let event = event_receiver.recv().await.unwrap();
        if is_to_me!(event.to, Service::Slack) {
            sender.send_message(&channel.clone().unwrap(), &event.message).unwrap();
        }
    }
}
