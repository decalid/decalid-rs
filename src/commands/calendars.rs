use anyhow::Result;
use chrono::{DateTime, Months, Utc};

use crate::{db::Db, events::model::DecalidEvent};

pub async fn list_calendars(db: &Db, user_id: i64) -> Result<()> {
    let calendars = db.list_calendars(user_id).await?;
    println!("Calendars:");
    for calendar in calendars {
        println!("{:?}", calendar);
    }

    Ok(())
}

pub async fn create_calendar(db: &Db, name: &str, user_id: i64, color: Option<&str>) -> Result<()> {
    let calendar = db.create_calendar(user_id, name, color).await?;
    println!("Calendar created: {:?}", calendar);
    Ok(())
}

pub async fn show_calendar(
    db: &Db,
    calendar_id: i64,
    min_date: Option<DateTime<Utc>>,
    max_date: Option<DateTime<Utc>>,
    max_results: i64,
) -> Result<()> {
    let calendar = db.admin().get_calendar_by_id(calendar_id).await;
    println!("Calendar: {:?}", calendar);

    println!(
        "Min date: {:?}\nMax date: {:?}\nMax results: {}",
        min_date, max_date, max_results
    );
    let min_date = min_date.unwrap_or_default();
    let max_date = max_date.unwrap_or_else(|| {
        min_date
            .checked_add_months(Months::new(12))
            .unwrap_or_default()
    });

    let mut events = db
        .get_current_events_between_dates(calendar_id, min_date, max_date, max_results)
        .await?;

    // Make repeat events be repeated
    events = events
        .into_iter()
        .flat_map(|event| {
            make_repeat_events_for_cli(event, max_date.with_timezone(&rrule::Tz::UTC))
        })
        .collect();
    events.sort_by_key(|event| event.inner.dtstart);
    events.truncate(max_results as usize);

    println!("Events:");
    for event in events {
        println!(
            "{} (st={}, from={:?} to={:?}, rr={})",
            event.summary.unwrap_or_default(),
            event.inner.status.unwrap_or_default(),
            event.inner.dtstart,
            event.inner.dtend,
            event.inner.rrule.unwrap_or_default()
        );
    }

    Ok(())
}

fn make_repeat_events_for_cli(
    event: DecalidEvent,
    last_date: DateTime<rrule::Tz>,
) -> Vec<DecalidEvent> {
    if event.recurrence_set.clone().into_iter().next().is_none() {
        vec![event]
    } else {
        event
            .recurrence_set
            .into_iter()
            .take_while(|p| p.lt(&last_date))
            .map(|dtstart| {
                let mut event = event.clone();
                event.inner.dtstart = Some(dtstart);
                if let Some(duration_str) = &event.inner.duration {
                    if let Some(dtstart) = event.inner.dtstart {
                        // Parse duration string like "PT1H" into Duration
                        if let Ok(duration) = iso8601_duration::Duration::parse(duration_str) {
                            if let Some(seconds) = duration.num_seconds() {
                                let chrono_duration = chrono::Duration::seconds(seconds as i64);
                                event.inner.dtend = Some(dtstart + chrono_duration);
                            }
                        }
                    }
                }
                event
            })
            .collect()
    }
}

pub async fn create_share_root(db: &Db, share_root: &str, owner_id: i64) -> Result<()> {
    Ok(db.shares(&share_root).admin_create(owner_id).await?)
}
pub async fn attach_calendar_to_share(db: &Db, share_root: &str, calendar_id: i64, description: &str) -> Result<String> {
    Ok(db.shares(&share_root).attach_calendar(calendar_id, description).await?)
}
