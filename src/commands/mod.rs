use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};

pub mod ics;
pub mod calendars;
pub mod users;


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}


#[derive(Subcommand)]
pub(crate) enum Commands {
    ImportICS {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        calendar_id: i64,
    },
    CreateUser {
        #[arg(short, long)]
        username: String,
    },
    ListUsers,
    CreateCalendar {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        user_id: i64,
        #[arg(short, long)]
        color: Option<String>,
    },
    ListCalendars {
        #[arg(short, long)]
        user_id: i64,
    },
    ShowCalendar {
        #[arg(short, long)]
        calendar_id: i64,

        #[arg(short = 's', long, default_value = "2023-01-01T00:00:00Z")]
        min_date: Option<DateTime<Utc>>,

        #[arg(short = 'e', long, default_value = "2025-01-01T00:00:00Z")]
        max_date: Option<DateTime<Utc>>,

        #[arg(short, long, default_value = "10")]
        max_results: i64,
    }
}