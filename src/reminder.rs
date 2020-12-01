use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};
use std::fs;
use tokio::sync::watch;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReminderType {
    // Different types of reminders are possible.
    // e.g. different reminders for the day before and one hour before.
    Void,
    OneHour, //TODO struct instead
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reminder {
    reminder_type: ReminderType,
    last_fire: DateTime<Local>,
}

#[derive(Serialize, Deserialize)]
pub struct Reminders {
    reminders: Vec<Reminder>,
}

impl Reminders {
    fn write(&self) {
        fs::write(
            std::path::Path::new("reminders.json"),
            serde_json::to_string_pretty(&self).expect("Can't serialize reminders"),
        )
        .expect("Can't write reminders.json")
    }
}

pub async fn handle(sender: watch::Sender<ReminderType>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000));

    loop {
        let now = Local::now();
        let next = next_meeting();
        let mut reminders = read_reminders();
        for mut reminder in &mut reminders.reminders {
            match reminder.reminder_type {
                ReminderType::OneHour => {
                    if in_remind_zone(now, next) && !in_remind_zone(reminder.last_fire, next) {
                        sender.broadcast(ReminderType::OneHour).unwrap();
                        reminder.last_fire = now;
                    }
                }
                _ => {}
            }
        }
        reminders.write();
        interval.tick().await;
    }
}

fn read_reminders() -> Reminders {
    match fs::read_to_string("reminders.json") {
        Ok(s) => serde_json::from_str(&s).expect("Error parsing reminders.json"),
        Err(_) => Reminders {
            reminders: vec![Reminder {
                reminder_type: ReminderType::OneHour,
                last_fire: Local::now(),
            }],
        },
    }
}

fn in_remind_zone(dt: DateTime<Local>, meeting: DateTime<Local>) -> bool {
    // Wether we're in a "send reminder"-zone.
    // Currently implemented as "are we 1 hour before?".
    ((meeting - Duration::hours(1))..meeting).contains(&dt)
}

fn next_meeting() -> DateTime<Local> {
    // Check current datetime and calculate when the next meeting is.
    let now = Local::now();
    let meeting_time = NaiveTime::from_hms(12, 15, 00);
    let meeting = match Datelike::weekday(&now) {
        Weekday::Thu => {
            // same day as meeting.
            // next week if meeting has occured.
            let date_delta = Duration::weeks(if now.time() < meeting_time { 0 } else { 1 });
            (now.date() + date_delta).and_time(meeting_time).unwrap()
        }
        _ => {
            let dow_index: i64 = now.date().weekday().num_days_from_monday().into();
            let date_delta = Duration::days((3 - dow_index).rem_euclid(7));
            (now.date() + date_delta).and_time(meeting_time).unwrap()
        }
    };
    assert!(meeting.weekday() == Weekday::Thu);
    meeting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_remind_zone() {
        let now = Local::now();
        assert!(super::in_remind_zone(now, now + Duration::minutes(30)));
        assert!(!super::in_remind_zone(now, now + Duration::hours(2)));
        assert!(!super::in_remind_zone(now, now - Duration::minutes(30)));
    }
}
