//! Example to download all messages and media from a chat.
//!
//! The `TG_ID` and `TG_HASH` environment variables must be set (learn how to do it for
//! [Windows](https://ss64.com/nt/set.html) or [Linux](https://ss64.com/bash/export.html))
//! to Telegram's API ID and API hash respectively.
//!
//! Then, run it as:
//!
//! ```sh
//! cargo run --example downloader -- CHAT_NAME
//! ```
//!
//! Messages will be printed to stdout, and media will be saved in the `target/` folder locally, named
//! message-[MSG_ID].[EXT]
//!

use crate::db::{setup_indexes, store_trade_db, TradeDocument};
use crate::parse_trade::{parse_trade, Trade};
use std::io::{BufRead, Write};
use std::{env, io};

use grammers_client::{Client, Config, SignInError};
use mime::Mime;
use mime_guess::mime;
use mongodb::{options::ClientOptions, Client as MongoClient};
use simple_logger::SimpleLogger;

use grammers_client::session::Session;
use grammers_client::types::Media;
use grammers_client::types::Media::{Contact, Document, Photo, Sticker};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const SESSION_FILE: &str = "downloader.session";

pub async fn async_main() -> Result<()> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    let api_id = env::var("TG_ID")
        .expect("TG_ID missing")
        .parse::<i32>()
        .expect("TG_ID invalid");
    let api_hash = env::var("TG_HASH").expect("TG_HASH missing");
    let group_name = env::args().nth(1).expect("group name missing");
    println!("Looking for group: {}", group_name);

    println!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(SESSION_FILE)?,
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await?;
    println!("Connected!");

    // If we can't save the session, sign out once we're done.
    let mut sign_out = false;

    if !client.is_authorized().await? {
        println!("Signing in...");
        let phone = prompt("Enter your phone number (international format): ")?;
        let token = client.request_login_code(&phone).await?;
        let code = prompt("Enter the code you received: ")?;
        let signed_in = client.sign_in(&token, &code).await;
        match signed_in {
            Err(SignInError::PasswordRequired(password_token)) => {
                // Note: this `prompt` method will echo the password in the console.
                //       Real code might want to use a better way to handle this.
                let hint = password_token.hint().unwrap();
                let prompt_message = format!("Enter the password (hint {}): ", &hint);
                let password = prompt(prompt_message.as_str())?;

                client
                    .check_password(password_token, password.trim())
                    .await?;
            }
            Ok(_) => (),
            Err(e) => panic!("{}", e),
        };
        println!("Signed in!");
        match client.session().save_to_file(SESSION_FILE) {
            Ok(_) => {}
            Err(e) => {
                println!("NOTE: failed to save the session, will sign out when done: {e}");
                sign_out = true;
            }
        }
    }

    println!("Finding group in your dialogs...");
    let mut dialogs = client.iter_dialogs();
    let mut target_chat = None;

    while let Some(dialog) = dialogs.next().await? {
        if dialog.chat().name().to_lowercase() == group_name.to_lowercase() {
            target_chat = Some(dialog.chat().clone());
            break;
        }
    }

    let chat = target_chat
        .unwrap_or_else(|| panic!("Group {} could not be found in your dialogs", group_name));

    let mut messages = client.iter_messages(chat.clone());

    // Create output file with chat name
    let output_file = "download.txt".to_string();
    let mut file = std::fs::File::create(&output_file)?;

    println!(
        "Group {} has {} total messages.",
        chat.name(),
        messages.total().await.unwrap()
    );

    let db_url = env::var("DB_URL").expect("DB_URL is not set");
    let db_name = env::var("DB_NAME").expect("DB_NAME is not set");
    let mongo_db_client_options = ClientOptions::parse(&db_url).await?;
    let mongo_db_client = MongoClient::with_options(mongo_db_client_options)?;
    let db = mongo_db_client.database(&db_name);
    let collection = db.collection::<TradeDocument>("trades");
    // Setup indexes
    setup_indexes(&collection).await?;

    let mut counter = 0;

    while let Some(msg) = messages.next().await? {
        counter += 1;
        let message_text = format!("Message {}:\n{}\n", msg.id(), msg.text());
        file.write_all(message_text.as_bytes())?;
        let trade = parse_trade(msg.text());
        let parsed = match &trade {
            Some(Trade::Open(trade)) => format!("{:?}\n", trade),
            Some(Trade::Close(trade)) => format!("{:?}\n", trade),
            None => "Failed to parse trade message\n".to_string(),
        };
        file.write_all(parsed.as_bytes())?;
        if let Some(trade) = trade {
            store_trade_db(&collection, trade, msg.id() as i64, msg.text().to_string()).await?;
        }
    }

    println!("Downloaded {counter} messages");
    println!("Messages saved to {}", output_file);

    if sign_out {
        // TODO revisit examples and get rid of "handle references" (also, this panics)
        drop(client.sign_out_disconnect().await);
    }

    Ok(())
}

fn get_file_extension(media: &Media) -> String {
    match media {
        Photo(_) => ".jpg".to_string(),
        Sticker(sticker) => get_mime_extension(sticker.document.mime_type()),
        Document(document) => get_mime_extension(document.mime_type()),
        Contact(_) => ".vcf".to_string(),
        _ => String::new(),
    }
}

fn get_mime_extension(mime_type: Option<&str>) -> String {
    mime_type
        .map(|m| {
            let mime: Mime = m.parse().unwrap();
            format!(".{}", mime.subtype())
        })
        .unwrap_or_default()
}

fn prompt(message: &str) -> Result<String> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(message.as_bytes())?;
    stdout.flush()?;

    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let mut line = String::new();
    stdin.read_line(&mut line)?;
    Ok(line)
}
