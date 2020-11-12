mod agenda;
mod discord;
mod slack;

use crate::agenda::{
    Agenda,
    AgendaPoint,
};
use futures::join;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let (from_discord, to_slack) = mpsc::unbounded_channel::<AgendaPoint>();
    let (from_slack, to_discord) = mpsc::unbounded_channel::<AgendaPoint>();

    join!(
        discord::handle(from_discord, to_discord),
        slack::handle(from_slack, to_slack),
    );
}

