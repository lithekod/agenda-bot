mod agenda;
mod discord;
mod slack;

use crate::agenda::AgendaPoint;

use futures::join;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let slack_token: Option<String> = None;
    let discord_token: Option<String> = None;

    println!("Hello, world!");

    let (from_discord, to_slack) = mpsc::unbounded_channel::<AgendaPoint>();
    let (from_slack, to_discord) = mpsc::unbounded_channel::<AgendaPoint>();

    join!(
        discord::handle(discord_token, from_discord, to_discord),
        slack::handle(slack_token, from_slack, to_slack),
    );
}

