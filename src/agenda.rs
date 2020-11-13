use serde::{
    Deserialize,
    Serialize,
};
use std::{
    fmt,
    fs,
};
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
        fs::write(std::path::Path::new("agenda.json"),
                serde_json::to_string_pretty(&self).expect("Can't serialize agenda"))
            .expect("Can't write agenda.json");
    }
}

pub enum ParseError {
    NoSuchCommand,
}

pub fn parse_message(
    message: &str,
    sender: &str,
    point_sender: &mpsc::UnboundedSender<AgendaPoint>
) -> Result<Option<String>, ParseError> {
    if message.starts_with("!add ") {
        let mut agenda = read_agenda();
        let agenda_point = AgendaPoint {
            title: message[5..].to_string(),
            adder: sender.to_string(),
        };
        point_sender.send(agenda_point.clone()).unwrap();
        agenda.points.push(agenda_point);
        agenda.write();
        Ok(None)
    } else if message.starts_with("!agenda") {
        Ok(Some(read_agenda()
                .points
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join("\n")))
    } else if message.starts_with("!clear") {
        Agenda {
            points: Vec::new(),
        }.write();
        Ok(None)
    } else if message.starts_with("!help") {
        Ok(Some("Available commands:\n  !add\n  !agenda\n  !clear\n  !help".to_string()))
    } else {
        Err(ParseError::NoSuchCommand)
    }
}

fn read_agenda() -> Agenda {
    serde_json::from_str::<Agenda>(
        &fs::read_to_string("agenda.json")
            .expect("Can't read agenda.json"))
        .expect("Error parsing agenda.json")
}
