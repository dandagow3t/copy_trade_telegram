use anyhow::Result;
use std::env;
use std::fmt;

#[derive(Debug)]
pub struct DbConfig {
    pub mongodb_uri: String,
    pub db_name: String,
}

impl fmt::Display for DbConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nDB Config:\n  mongodb_uri: {}\n  db_name: {}",
            self.mongodb_uri, self.db_name
        )
    }
}

#[derive(Debug)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: String,
    pub group_name: String,
    pub pool_frequency: u64,
}

impl fmt::Display for TelegramConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nTelegram Config:\n  group_name: {}\n  pool_frequency: {} s",
            self.group_name, self.pool_frequency
        )
    }
}

#[derive(Debug)]
pub struct TradingConfig {
    pub trade_on: bool,
    pub position_size_sol: f64,
    pub slippage_bps: u16,
    pub filter_strategy: String,
}

impl fmt::Display for TradingConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nTrading Config:\n  trade_on: {}\n  position_size_sol: {}\n  slippage_bps: {}\n  filter_strategy: {}",
            self.trade_on, self.position_size_sol, self.slippage_bps, self.filter_strategy
        )
    }
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
            pool_frequency: env::var("TG_POOL_FREQUENCY")
                .expect("TG_POOL_FREQUENCY not set.")
                .parse()?,
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
            filter_strategy: env::var("FILTER_STRATEGY").expect("FILTER_STRATEGY not set."),
        })
    }
}
