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

use crate::tg_copy::db::{self, TradeDocument};
use crate::tg_copy::parse_trade::{parse_trade, Trade, TradeClose, TradeOpen};
use crate::trade::meme_trader::MemeTrader;
use grammers_client::types::Chat;
use grammers_client::{Client, Config, SignInError};
use grammers_session::Session;
use mongodb::Collection;
use std::io::BufRead;
use std::sync::Arc;
use std::{env, time::Duration};
use tokio::time;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const SESSION_FILE: &str = "downloader.session";

pub async fn async_main() -> Result<()> {
    // Connect to MongoDB
    let mongodb_uri = env::var("MONGODB_URI").expect("MONGODB_URI not set.");
    let client = mongodb::Client::with_uri_str(&mongodb_uri).await?;
    let db_name = env::var("DB_NAME").expect("DB_NAME not set.");
    let db = client.database(&db_name);
    let collection = db.collection::<TradeDocument>("trades");

    // Setup indexes
    db::setup_indexes(&collection).await?;

    // Connect to Telegram
    let api_id = env::var("TG_ID").expect("TG_ID not set.").parse()?;
    let api_hash = env::var("TG_HASH").expect("TG_HASH not set.");
    let group_name = env::var("GROUP_NAME").expect("GROUP_NAME not set.");

    println!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(SESSION_FILE)?,
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await?;

    if !client.is_authorized().await? {
        println!("First time setup - need to log in!");
        handle_login(&client).await?;
    }
    println!("Connected!");

    // Find the target group
    let chat = find_group(&client, &group_name).await?;

    // Get last processed message ID
    let last_message_id = db::get_last_message_id(&collection).await?.unwrap_or(0);
    println!("Starting from message ID: {}", last_message_id);

    // Process historical messages first
    process_historical_messages(&client, &collection, &chat, last_message_id).await?;

    // Then start listening for new messages
    println!("Listening for new messages...");
    listen_for_new_messages(&client, &collection, &chat).await?;

    Ok(())
}

async fn handle_login(client: &Client) -> Result<()> {
    println!("Signing in...");
    let phone = prompt("Enter your phone number (international format): ")?;
    let token = client.request_login_code(&phone).await?;
    let code = prompt("Enter the code you received: ")?;
    let signed_in = client.sign_in(&token, &code).await;

    match signed_in {
        Err(SignInError::PasswordRequired(password_token)) => {
            let hint = password_token.hint().unwrap_or("none");
            let prompt_message = format!("Enter the password (hint {}): ", hint);
            let password = prompt(&prompt_message)?;
            client
                .check_password(password_token, password.trim())
                .await?;
        }
        Ok(_) => (),
        Err(e) => return Err(e.into()),
    }

    println!("Signed in!");
    client.session().save_to_file(SESSION_FILE)?;
    Ok(())
}

async fn find_group(client: &Client, group_name: &str) -> Result<Chat> {
    println!("Finding group {}...", group_name);
    let mut dialogs = client.iter_dialogs();

    while let Some(dialog) = dialogs.next().await? {
        if dialog.chat().name().to_lowercase() == group_name.to_lowercase() {
            return Ok(dialog.chat().clone());
        }
    }

    Err("Group not found in your dialogs".into())
}

async fn process_historical_messages(
    client: &Client,
    collection: &Collection<TradeDocument>,
    chat: &Chat,
    last_message_id: i64,
) -> Result<()> {
    let mut messages = client.iter_messages(chat.clone());
    while let Some(message) = messages.next().await? {
        if (message.id() as i64) <= last_message_id {
            break;
        }
        let text = message.text();
        println!("Processing message {} - {}", message.id(), text);
        if let Some(trade) = parse_trade(text) {
            db::store_trade_db(
                collection,
                trade,
                message.id() as i64,
                text.to_string(),
                message.date().into(),
            )
            .await?;
            println!("Store message {}", message.id());
        }
    }
    Ok(())
}

async fn listen_for_new_messages(
    client: &Client,
    collection: &Collection<TradeDocument>,
    chat: &Chat,
) -> Result<()> {
    let trader = Arc::new(MemeTrader::new());
    let mut interval = time::interval(Duration::from_secs(10));
    let mut counter = 0;
    loop {
        interval.tick().await;
        counter += 1;
        if counter % 30 == 0 {
            println!("#{} check", counter);
        }
        let last_message_id = db::get_last_message_id(collection).await?.unwrap_or(0);
        let mut messages = client.iter_messages(chat.clone());

        while let Some(message) = messages.next().await? {
            if (message.id() as i64) <= last_message_id {
                break;
            }

            let text = message.text();
            if let Some(trade) = parse_trade(text) {
                let trade_clone = trade.clone();
                let collection_clone = collection.clone();
                let message_id = message.id() as i64;
                let text_clone = text.to_string();
                let message_date = message.date();
                let trader = Arc::clone(&trader);

                // Spawn DB storage task
                let db_task = tokio::spawn(async move {
                    db::store_trade_db(
                        &collection_clone,
                        trade_clone,
                        message_id,
                        text_clone,
                        message_date.into(),
                    )
                    .await
                });

                // Spawn trading task
                let trade_task = tokio::spawn(async move {
                    match &trade {
                        Trade::Open(open_trade) => {
                            tracing::info!("It's buy");
                            if open_trade.strategy == "prodybb120sec" {
                                let tx_sig = trader
                                    .buy_pump_fun(open_trade.contract_address.as_str(), 0.05, 500)
                                    .await?;
                                tracing::info!("tx sig: {}", tx_sig);
                            }
                        }
                        Trade::Close(close_trade) => {
                            tracing::info!("It's sell");
                            if close_trade.strategy == "prodybb120sec" {
                                let tx_sig = trader
                                    .sell_pump_fun(close_trade.contract_address.as_str(), 100000)
                                    .await?;
                                tracing::info!("tx sig: {}", tx_sig);
                            }
                        }
                    }
                    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                });

                // join both tasks
                let _ = tokio::join!(db_task, trade_task);
            }
        }
    }
}

fn prompt(message: &str) -> Result<String> {
    use std::io::{self, Write};

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(message.as_bytes())?;
    stdout.flush()?;

    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let mut line = String::new();
    stdin.read_line(&mut line)?;
    Ok(line.trim().to_string())
}
