use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::models::{Calendar, Event};

pub async fn list_calendars(pool: &SqlitePool, user_id: i64) -> Result<()> {
    let calendars = sqlx::query_as::<_, Calendar>("SELECT * FROM calendars WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await?;

    println!("Calendars:");
    for calendar in calendars {
        println!("{:?}", calendar);
    }

    Ok(())
}

pub async fn create_calendar(
    pool: &SqlitePool,
    name: &str,
    user_id: i64,
    color: Option<&str>,
) -> Result<()> {
    let calendar = sqlx::query_as::<_, Calendar>(
        "INSERT INTO calendars (user_id, name, color) VALUES (?, ?, ?) RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(color)
    .fetch_one(pool)
    .await?;

    println!("Calendar created: {:?}", calendar);

    Ok(())
}

pub async fn show_calendar(
    pool: &SqlitePool,
    calendar_id: i64,
    min_date: Option<DateTime<Utc>>,
    max_date: Option<DateTime<Utc>>,
    max_results: i64,
) -> Result<()> {
    let calendar = sqlx::query_as::<_, Calendar>("SELECT * FROM calendars WHERE id = ?")
        .bind(calendar_id)
        .fetch_one(pool)
        .await?;

    println!("Calendar: {:?}", calendar);

    println!("Min date: {:?}\nMax date: {:?}\nMax results: {}", min_date, max_date, max_results);

    let events = sqlx::query_as::<_, Event>(
        "SELECT * FROM events e INNER JOIN event_versions ev ON e.current_version_id = ev.id WHERE calendar_id = ? AND dtstart >= ? AND dtend <= ? LIMIT ?",
    )
    .bind(calendar_id)
    .bind(min_date)
    .bind(max_date)
    .bind(max_results)
    .fetch_all(pool)
    .await?;

    println!("Events:");
    for event in events {
        println!("{:?}", event);
    }

    Ok(())
}
