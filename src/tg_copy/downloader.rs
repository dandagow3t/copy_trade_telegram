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

use crate::config::{DbConfig, TelegramConfig, TradingConfig};
use crate::signer::SignerContext;
use crate::solana::balance::get_ata_balance;
use crate::solana::util::env;
use crate::tg_copy::db::{self, TradeDocument};
use crate::tg_copy::parse_trade::{parse_trade, Trade};
use crate::trade::meme_trader::MemeTrader;
use anyhow::Result;
use grammers_client::types::Chat;
use grammers_client::{Client, Config, SignInError};
use grammers_session::Session;
use mongodb::Collection;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time;

const SESSION_FILE: &str = "downloader.session";

#[derive(Debug)]
struct TradeMemory {
    last_trade_time: u64,
    strategy: String,
}

pub async fn async_main() -> Result<()> {
    // Load configurations
    let db_config = DbConfig::from_env()?;
    let telegram_config = TelegramConfig::from_env()?;
    let trading_config = TradingConfig::from_env()?;

    // Print configs
    tracing::info!("{}", db_config);
    tracing::info!("{}", telegram_config);
    tracing::info!("{}", trading_config);

    // Connect to MongoDB
    let client = mongodb::Client::with_uri_str(&db_config.mongodb_uri).await?;
    let db = client.database(&db_config.db_name);
    let collection = db.collection::<TradeDocument>("trades");

    // Setup indexes
    db::setup_indexes(&collection).await?;

    // Connect to Telegram
    tracing::info!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(SESSION_FILE)?,
        api_id: telegram_config.api_id,
        api_hash: telegram_config.api_hash.clone(),
        params: Default::default(),
    })
    .await?;

    if !client.is_authorized().await? {
        tracing::info!("First time setup - need to log in!");
        handle_login(&client).await?;
    }
    tracing::info!("Connected!");

    // Find the target group
    let chat = find_group(&client, &telegram_config.group_name).await?;

    // Get last processed message ID
    let last_message_id = db::get_last_message_id(&collection).await?.unwrap_or(0);
    tracing::info!("Starting from message ID: {}", last_message_id);

    // Process historical messages first
    process_historical_messages(&client, &collection, &chat, last_message_id).await?;

    // Then start listening for new messages

    listen_for_new_messages(
        &client,
        &collection,
        &chat,
        trading_config.filter_strategies,
        trading_config.position_size_sol,
        trading_config.slippage_bps,
        telegram_config.pool_frequency,
        trading_config.trade_on,
        trading_config.strategy_filter_on,
    )
    .await?;

    Ok(())
}

async fn handle_login(client: &Client) -> Result<()> {
    tracing::info!("Signing in...");
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

    tracing::info!("Signed in!");
    client.session().save_to_file(SESSION_FILE)?;
    Ok(())
}

async fn find_group(client: &Client, group_name: &str) -> Result<Chat> {
    tracing::info!("Finding group {}...", group_name);
    let mut dialogs = client.iter_dialogs();

    while let Some(dialog) = dialogs.next().await? {
        if dialog.chat().name().to_lowercase() == group_name.to_lowercase() {
            return Ok(dialog.chat().clone());
        }
    }

    Err(anyhow::anyhow!("Group not found in your dialogs"))
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
        tracing::info!("Processing message {} - {}", message.id(), text);
        if let Some(trade) = parse_trade(text) {
            db::store_trade_db(
                collection,
                trade,
                message.id() as i64,
                text.to_string(),
                message.date().into(),
            )
            .await?;
            tracing::info!("Store message {}", message.id());
        }
    }
    Ok(())
}

async fn listen_for_new_messages(
    client: &Client,
    collection: &Collection<TradeDocument>,
    chat: &Chat,
    filter_strategies: Vec<String>,
    position_size_sol: f64,
    slippage_bps: u16,
    pool_frequency: u64,
    execute: bool,
    strategy_filter_on: bool,
) -> Result<()> {
    let trader = Arc::new(MemeTrader::new());
    let trade_memory: Arc<Mutex<HashMap<String, TradeMemory>>> =
        Arc::new(Mutex::new(HashMap::new()));
    const TRADE_TIMEOUT_SECS: u64 = 300;

    tracing::info!(
        "Strategy filtering is {}",
        if strategy_filter_on { "ON" } else { "OFF" }
    );

    let mut interval = time::interval(Duration::from_secs(pool_frequency));
    let mut counter = 0;
    tracing::info!("Listening for new messages...\n");
    loop {
        interval.tick().await;
        if counter % 30 == 0 {
            tracing::info!(".");
        } else {
            print!(".");
            std::io::stdout().flush().unwrap();
        }
        counter += 1;

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
                let trade_memory = Arc::clone(&trade_memory);

                // Get current signer before spawning tasks
                let signer = SignerContext::current().await;

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

                if execute {
                    let filter_strategies_clone = filter_strategies.clone();
                    let trade_task = tokio::spawn(SignerContext::with_signer(signer, async move {
                        match &trade {
                            Trade::Open(open_trade) => {
                                tracing::info!(
                                    "Buy signal received: {}, {}, {}",
                                    open_trade.token,
                                    open_trade.strategy,
                                    open_trade.contract_address
                                );

                                let should_execute = {
                                    let memory = trade_memory.lock().await;
                                    let current_time = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs();

                                    if let Some(last_trade) =
                                        memory.get(&open_trade.contract_address)
                                    {
                                        if current_time - last_trade.last_trade_time
                                            > TRADE_TIMEOUT_SECS
                                        {
                                            true
                                        } else {
                                            tracing::info!(
                                                "Skipping duplicate trade for {} (previous strategy: {})",
                                                open_trade.token,
                                                last_trade.strategy
                                            );
                                            false
                                        }
                                    } else {
                                        true
                                    }
                                };

                                // Modified strategy check to respect STRATEGY_FILTER_ON
                                let strategy_check = if strategy_filter_on {
                                    filter_strategies_clone
                                        .iter()
                                        .any(|s| s == &open_trade.strategy)
                                } else {
                                    true
                                };

                                if should_execute && strategy_check {
                                    match trader
                                        .buy_pump_fun(
                                            open_trade.contract_address.as_str(),
                                            position_size_sol,
                                            slippage_bps,
                                        )
                                        .await
                                    {
                                        Ok(tx_sig) => {
                                            let mut memory = trade_memory.lock().await;
                                            memory.insert(
                                                open_trade.contract_address.clone(),
                                                TradeMemory {
                                                    last_trade_time: SystemTime::now()
                                                        .duration_since(UNIX_EPOCH)
                                                        .unwrap()
                                                        .as_secs(),
                                                    strategy: open_trade.strategy.clone(),
                                                },
                                            );
                                            tracing::info!(
                                                "Buy tx: https://solscan.io/tx/{}",
                                                tx_sig
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!("Buy transaction failed: {:?}", e);
                                        }
                                    }
                                }
                            }
                            Trade::Close(close_trade) => {
                                tracing::info!(
                                    "Sell, {}, {}, {}",
                                    close_trade.token,
                                    close_trade.strategy,
                                    close_trade.contract_address
                                );

                                // Modified strategy check for close trades
                                let strategy_check = if strategy_filter_on {
                                    filter_strategies_clone
                                        .iter()
                                        .any(|s| s == &close_trade.strategy)
                                } else {
                                    true
                                };

                                if strategy_check {
                                    // get account holdings for contract address
                                    let owner = Pubkey::from_str(
                                        "9AFb3BJTybJVvjWejqxstz9DUwYQxPepT94VCBi4escf",
                                    )
                                    .unwrap();
                                    let holdings = get_ata_balance(
                                        &RpcClient::new(env("SOLANA_RPC_URL")),
                                        &owner,
                                        &Pubkey::from_str(close_trade.contract_address.as_str())?,
                                    )
                                    .await
                                    .unwrap();
                                    tracing::info!("holdings: {:?}", holdings);
                                    match trader
                                        .sell_pump_fun(
                                            close_trade.contract_address.as_str(),
                                            holdings.parse::<u64>()?,
                                        )
                                        .await
                                    {
                                        Ok(tx_sig) => {
                                            tracing::info!(
                                                "Sell tx: https://solscan.io/tx/{}",
                                                tx_sig
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!("Sell transaction failed: {:?}", e);
                                        }
                                    }
                                }
                                let mut memory = trade_memory.lock().await;
                                memory.remove(&close_trade.contract_address);
                            }
                        }
                        Ok(())
                    }));

                    // join both tasks
                    let _ = tokio::join!(db_task, trade_task);
                }
            }
        }
    }
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
    Ok(line.trim().to_string())
}
