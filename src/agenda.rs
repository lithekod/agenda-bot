use serde::{Deserialize, Serialize};
use std::{fmt, fs};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgendaPoint {
    title: String,
    adder: String,
}

impl fmt::Display for AgendaPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.title, self.adder)
    }
}

impl AgendaPoint {
    pub fn to_add_message(&self) -> String {
        format!("'{}' added by {}", self.title, self.adder)
    }
}

#[derive(Deserialize, Serialize)]
pub struct Agenda {
    points: Vec<AgendaPoint>,
}

impl Agenda {
    fn write(&self) {
        fs::write(
            std::path::Path::new("agenda.json"),
            serde_json::to_string_pretty(&self).expect("Can't serialize agenda"),
        )
        .expect("Can't write agenda.json");
    }
}

impl fmt::Display for Agenda {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self
            .points
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        write!(
            f,
            "{}",
            match s.as_str() {
                "" => "Empty agenda",
                _ => &s,
            }
        )
    }
}

pub enum Emoji {
    Ok,
    Confused,
    Err,
}

pub fn parse_message<F>(
    message: &str,
    sender: &str,
    send_message: F,
    point_sender: &mpsc::UnboundedSender<AgendaPoint>,
) -> Option<Emoji>
where
    F: FnOnce(String),
{
    if message.starts_with("!add ") {
        let mut agenda = read_agenda();
        let agenda_point = AgendaPoint {
            title: message[5..].to_string(),
            adder: sender.to_string(),
        };
        point_sender.send(agenda_point.clone()).unwrap();
        agenda.points.push(agenda_point);
        agenda.write();
        Some(Emoji::Ok)
    } else if message.starts_with("!agenda") {
        send_message(read_agenda().to_string());
        None
    } else if message.starts_with("!clear") {
        Agenda { points: Vec::new() }.write();
        Some(Emoji::Ok)
    } else if message.starts_with("!help") {
        send_message("Available commands:\n```!add    -- Add something\n!agenda -- Print the agenda\n!clear  -- Remove all items\n!help```".to_string());
        None
    } else if message.starts_with("!") {
        Some(Emoji::Confused)
    } else {
        Some(Emoji::Err)
    }
}

pub fn read_agenda() -> Agenda {
    serde_json::from_str::<Agenda>(
        &fs::read_to_string("agenda.json").expect("Can't read agenda.json"),
    )
    .expect("Error parsing agenda.json")
}
