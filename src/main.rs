mod agenda;
mod discord;
mod reminder;
mod slack;

use futures::join;
use tokio::sync::{mpsc, oneshot};

const HELP_MESSAGE: &'static str = "Available commands:\n```!add    -- Add something\n!agenda -- Print the agenda\n!clear  -- Remove all items\n!help```";

#[derive(Clone, Copy, Debug)]
pub enum Service {
    Discord,
    Slack,
}

#[derive(Clone, Copy, Debug)]
pub enum To {
    All,
    Not(Service),
    Only(Service),
}

#[macro_export]
macro_rules! is_to_me {
    ($to:expr, $service:pat) => {
        match $to {
            To::All | To::Only($service) => true,
            To::Only(_) => false,
            To::Not($service) => false,
            To::Not(_) => true
        }
    }
}

mod kodapa {
    use crate::{Service, To, agenda::{Agenda, AgendaPoint, read_agenda}, mpsc, oneshot};

    //TODO actual Result
    #[derive(Debug)]
    pub enum Feedback {
        Ok,
        // Err,
    }

    #[derive(Debug)]
    pub struct Request {
        pub origin: Service,
        pub message: String,
        pub sender: String,
        pub feedback: Option<oneshot::Sender<Feedback>>,
    }

    #[derive(Debug)]
    pub struct Event {
        pub to: To,
        pub message: String,
    }

    pub struct Kodapa {
        event_receiver: mpsc::UnboundedReceiver<Request>,
        event_senders: Vec<mpsc::UnboundedSender<Event>>,
    }

    impl Kodapa {
        pub fn new(
            event_receiver: mpsc::UnboundedReceiver<Request>,
            event_senders: Vec<mpsc::UnboundedSender<Event>>,
        ) -> Self {
            Self { event_receiver, event_senders }
        }

        fn send_message(&mut self, to: To, message: String) {
            for sender in &self.event_senders {
                sender.send(Event{ to, message: message.clone() }).unwrap();
            }
        }

        fn add(&mut self, origin: Service, point: AgendaPoint, feedback: Option<oneshot::Sender<Feedback>>) {
            let message = point.to_add_message().to_string();
            let mut agenda = read_agenda();
            agenda.points.push(point);
            agenda.write();
            self.send_message(To::Not(origin), message);
            if let Some(feedback) = feedback {
                feedback.send(Feedback::Ok).unwrap();
            }
        }

        fn agenda(&mut self, origin: Service) {
            self.send_message(To::Only(origin), read_agenda().to_string());
        }

        fn help(&mut self, origin: Service) {
            self.send_message(To::Only(origin), crate::HELP_MESSAGE.to_string());
        }

        fn clear(&mut self, sender: String) {
            Agenda { points: Vec::new() }.write();
            self.send_message(To::All, format!("Agenda cleared by {}", sender));
        }

        pub async fn handle(&mut self) {
            loop {
                let request = self.event_receiver.recv().await.unwrap();
                match request.message.split(" ").next() { //TODO unwrap?
                    Some("!add") => self.add(
                        request.origin,
                        AgendaPoint{
                            title: request.message[5..].to_string(),
                            adder: request.sender,
                        },
                        request.feedback,
                    ),
                    Some("!agenda") => self.agenda(request.origin),
                    Some("!help") => self.help(request.origin),
                    Some("!clear") => self.clear(request.sender),
                    _ => {
                        if request.message.starts_with("!") {
                            // ...
                        }
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let (request_sender, request_receiver) = mpsc::unbounded_channel::<kodapa::Request>();
    let (discord_sender, discord_receiver) = mpsc::unbounded_channel::<kodapa::Event>();
    let (slack_sender, slack_receiver) = mpsc::unbounded_channel::<kodapa::Event>();

    let mut kodapa = kodapa::Kodapa::new(request_receiver, vec!(discord_sender, slack_sender));

    let _ = join!(
        kodapa.handle(),
        discord::handle(request_sender.clone(), discord_receiver),
        slack::handle(request_sender.clone(), slack_receiver),
        //reminder::handle(request_sender),
    );
}
