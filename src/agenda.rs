use serde::{Deserialize, Serialize};
use std::{fmt, fs};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgendaPoint {
    pub title: String,
    pub adder: String,
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
    pub points: Vec<AgendaPoint>,
}

impl Agenda {
    pub fn write(&self) {
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

pub fn read_agenda() -> Agenda {
    match fs::read_to_string("agenda.json") {
        Ok(s) => serde_json::from_str(&s).expect("Error parsing agenda.json"),
        Err(_) => Agenda { points: Vec::new() },
    }
}
