use std::env;

use anyhow::Result;

#[derive(Debug)]
pub struct DbConfig {
    pub mongodb_uri: String,
    pub db_name: String,
}

#[derive(Debug)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: String,
    pub group_name: String,
}

#[derive(Debug)]
pub struct TradingConfig {
    pub trade_on: bool,
    pub position_size_sol: f64,
    pub slippage_bps: u16,
}

impl DbConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            mongodb_uri: env::var("MONGODB_URI").expect("MONGODB_URI not set."),
            db_name: env::var("DB_NAME").expect("DB_NAME not set."),
        })
    }
}

impl TelegramConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api_id: env::var("TG_ID").expect("TG_ID not set.").parse()?,
            api_hash: env::var("TG_HASH").expect("TG_HASH not set."),
            group_name: env::var("GROUP_NAME").expect("GROUP_NAME not set."),
        })
    }
}

impl TradingConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            trade_on: env::var("TRADE_ON")
                .expect("TRADE_ON not set.")
                .to_lowercase()
                == "true",
            position_size_sol: env::var("POSITION_SIZE_SOL")
                .expect("POSITION_SIZE_SOL not set.")
                .parse()?,
            slippage_bps: env::var("SLIPPAGE_BPS")
                .expect("SLIPPAGE_BPS not set.")
                .parse()?,
        })
    }
}
