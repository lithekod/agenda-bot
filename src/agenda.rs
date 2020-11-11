use serde::{
    Deserialize,
    Serialize,
};
use std::{
    fmt,
    fs,
};

#[derive(Debug, Deserialize, Serialize)]
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
    points: Vec<AgendaPoint>,
}

pub fn read_agenda() -> Agenda {
    serde_json::from_str::<Agenda>(
        &fs::read_to_string("agenda.json")
            .expect("Can't read agenda.json"))
        .expect("Error parsing agenda.json")
}

pub fn write_agenda(agenda: Agenda) {
    fs::write(std::path::Path::new("agenda.json"),
              serde_json::to_string_pretty(&agenda).expect("Can't serialize agenda"))
        .expect("Can't write agenda.json");
}

pub fn add_point(point: AgendaPoint) {
    let mut agenda = read_agenda();
    agenda.points.push(point);
    write_agenda(agenda);
}
