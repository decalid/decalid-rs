use std::str::FromStr as _;

use anyhow::Result;
use clap::Parser;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use tokio::task_local;

use decalid::config::Config;
use decalid::db::Db;
use decalid::ics;
use decalid::telemetry;

// The commands module is specific to the binary and not part of the library
mod commands;

use commands::{
    caldav::{add_caldav_source, sync_caldav_calendar},
    calendars::{
        attach_calendar_to_share, create_calendar, create_share_root, list_calendars, show_calendar,
    },
    jwt::{login_new_device, logout_device},
    timezones::{import_timezone, list_timezones, set_calendar_timezone, show_timezone},
    users::{create_user, list_users},
    Cli, Commands,
};

task_local! {
    static LOCAL_DB: Db;
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load the configuration
    let config = match Config::from_file("config.json") {
        Ok(config) => {
            println!("Loaded configuration from file");
            config
        }
        Err(e) => {
            println!("Failed to load configuration from file, using environment variables: {e}");
            Config::from_env()
        }
    };

    // Initialize telemetry
    telemetry::init(&config.telemetry.log_level);

    // Connect to database
    let connection_options = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
    let pool = SqlitePool::connect_with(connection_options).await?;

    // Migrate database
    sqlx::migrate!().run(&pool).await?;

    let mut db = Db::new(pool);

    match &cli.command {
        Commands::ImportICS { file, calendar_id } => {
            ics::import_ics(&db, *calendar_id, file).await?;
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
        Commands::CreateShareRoot {
            share_name,
            owner_id,
        } => {
            create_share_root(&db, share_name, *owner_id).await?;
        }
        Commands::AddCalendarToShare {
            share_root,
            calendar_id,
            description,
        } => {
            attach_calendar_to_share(&db, share_root, *calendar_id, description).await?;
        }
        Commands::AddCalDavSource {
            calendar_id,
            url,
            username,
            password,
            token,
        } => {
            add_caldav_source(
                &mut db,
                *calendar_id,
                url,
                username.as_deref(),
                password.as_deref(),
                token.as_deref(),
            )
            .await?;
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
        Commands::LoginNewDevice {
            user_id,
            device_description,
            override_expiration,
        } => {
            login_new_device(
                &config,
                &db,
                *user_id,
                device_description,
                *override_expiration,
            )
            .await?;
        }
        Commands::LogoutDevice { user_id, device_id } => {
            logout_device(&db, *user_id, device_id).await?;
        }
        Commands::Server { port } => {
            let server_port = port.unwrap_or(config.server.port);
            let config = Config {
                server: decalid::config::ServerConfig {
                    port: server_port,
                    ..config.server
                },
                ..config
            };
            println!("Starting server on port {server_port}");
            decalid::api::start_server(&config, db).await?;
            return Ok(());
        }
    }
    db.close().await;
    Ok(())
}
