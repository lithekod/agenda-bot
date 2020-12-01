mod agenda;
mod discord;
mod reminder;
mod slack;

use crate::agenda::AgendaPoint;
use crate::reminder::ReminderType;
use futures::join;
use tokio::sync::{mpsc, watch};

#[tokio::main]
async fn main() {
    let (from_discord, to_slack) = mpsc::unbounded_channel::<AgendaPoint>();
    let (from_slack, to_discord) = mpsc::unbounded_channel::<AgendaPoint>();

    let (reminder_sender, reminder_receiver) = watch::channel(ReminderType::Void);

    join!(
        reminder::handle(reminder_sender),
        discord::handle(from_discord, to_discord, reminder_receiver.clone()),
        slack::handle(from_slack, to_slack, reminder_receiver),
    );
}
