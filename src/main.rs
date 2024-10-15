use anyhow::Result;
use clap::Parser;
use sqlx::sqlite::SqlitePool;

mod commands;
use commands::{calendars::{create_calendar, list_calendars, show_calendar}, ics::import_ics, users::{create_user, list_users}, Cli, Commands};
mod models;




#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let pool = SqlitePool::connect("sqlite:db.sqlite").await?;

    match &cli.command {
        Commands::ImportICS { file, calendar_id } => {
            import_ics(&pool, *calendar_id, file).await?;
        }
        Commands::CreateUser { username } => {
            create_user(&pool, username).await?;
        }
        Commands::ListUsers => {
            list_users(&pool).await?;
        }
        Commands::CreateCalendar {
            name,
            user_id,
            color,
        } => {
            create_calendar(&pool, name, *user_id, color.as_deref()).await?;
        }
        Commands::ListCalendars { user_id } => {
            list_calendars(&pool, *user_id).await?;
        }
        Commands::ShowCalendar {
            calendar_id,
            min_date,
            max_date,
            max_results,
        } => {
            show_calendar(&pool, *calendar_id, *min_date, *max_date, *max_results).await?;
        }
    }

    pool.close().await;
    Ok(())
}

