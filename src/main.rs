use anyhow::Result;
use clap::Parser;
use db::Db;
use sqlx::sqlite::SqlitePool;

mod api;
mod auth;
mod cache;
mod caldav;
mod chrono_utils;
mod commands;
mod config;
mod db;
mod events;
mod ics;
mod telemetry;
mod timezone;
mod transformation;

use commands::{
    caldav::{add_caldav_source, sync_caldav_calendar}, calendars::{attach_calendar_to_share, create_calendar, create_share_root, list_calendars, show_calendar}, timezones::{import_timezone, list_timezones, set_calendar_timezone, show_timezone}, users::{create_user, list_users}, Cli, Commands
};
use config::Config;
use ics::import_ics;
use tokio::task_local;

task_local! {
    static LOCAL_DB: Db;
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Load configuration
    let config = if let Ok(config) = Config::from_file("config.json") {
        config
    } else {
        Config::from_env()
    };
    
    // Initialize telemetry
    telemetry::init(&config.telemetry.log_level);
    
    // Connect to database
    let pool = SqlitePool::connect(&config.database_url).await?;
    let mut db = Db::new(pool);

    match &cli.command {
        Commands::ImportICS { file, calendar_id } => {
            import_ics(&db, *calendar_id, file).await?;
        }
        Commands::CreateUser { username } => {
            create_user(&db, username).await?;
        }
        Commands::ListUsers => {
            list_users(&db).await?;
        }
        Commands::CreateCalendar {
            name,
            user_id,
            color,
        } => {
            create_calendar(&db, name, *user_id, color.as_deref()).await?;
        }
        Commands::ListCalendars { user_id } => {
            list_calendars(&db, *user_id).await?;
        }
        Commands::ShowCalendar {
            calendar_id,
            min_date,
            max_date,
            max_results,
        } => {
            show_calendar(&db, *calendar_id, *min_date, *max_date, *max_results).await?;
        }
        Commands::CreateShareRoot { share_name, owner_id } => {
            create_share_root(&db, share_name, *owner_id).await?;
        }
        Commands::AddCalendarToShare { share_root, calendar_id, description } => {
            attach_calendar_to_share(&db, share_root, *calendar_id, description).await?;
        }
        Commands::AddCalDavSource { calendar_id, url, username, password, token } => {
            add_caldav_source(&mut db, *calendar_id, url, username.as_deref(), password.as_deref(), token.as_deref()).await?;
        }
        Commands::SyncCalDavCalendar { calendar_id } => {
            sync_caldav_calendar(&mut db, *calendar_id).await?;
        }
        Commands::ListTimezones => {
            list_timezones(&db).await?;
        }
        Commands::SetCalendarTimezone { calendar_id, tzid } => {
            set_calendar_timezone(&db, *calendar_id, tzid).await?;
        }
        Commands::ShowTimezone { tzid } => {
            show_timezone(&db, tzid).await?;
        }
        Commands::ImportTimezone { file } => {
            import_timezone(&db, file).await?;
        }
        Commands::Server { port } => {
            // Use the port from the command line if provided, otherwise use the one from the config
            let port = port.unwrap_or(config.server.port);

            let config = config.with_port(port);
            
            // Start the unified API server which includes both REST and CalDAV endpoints
            api::start_server(&config, db).await?;
            
            // Return without closing the database since the server will handle that
            return Ok(());
        }
    }

    Ok(db.close().await)
}
