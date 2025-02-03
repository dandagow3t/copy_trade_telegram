use crate::parse_trade::Trade;
use anyhow::Result;
use futures::stream::TryStreamExt;
use mongodb::{
    bson::doc,
    options::IndexOptions,
    Collection, IndexModel,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum TradeType {
    Open,
    Close,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeDocument {
    pub message_id: i64,
    pub strategy: String,
    pub token: String,
    pub contract_address: String,
    pub trade_type: TradeType,
    pub original_message: String,
    pub op_type: Option<String>,  // null for Open trades
    pub buy_price: Option<f64>,   // used for Open trades
    pub num_buys: Option<u32>,    // used for Open trades
    pub total_buys: Option<f64>,  // used for Open trades
    pub time_window: Option<u32>, // used for Open trades
    pub market_cap: Option<f64>,  // used for Open trades
    pub entry_price: Option<f64>, // used for Close trades
    pub exit_price: Option<f64>,  // used for Close trades
    pub profit_pct: Option<f64>,  // used for Close trades
}

pub async fn setup_indexes(collection: &Collection<TradeDocument>) -> Result<()> {
    // Create indexes
    let message_id_index = IndexModel::builder()
        .keys(doc! { "message_id": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build();

    let strategy_token_index = IndexModel::builder()
        .keys(doc! { "strategy": 1, "token": 1 })
        .build();

    collection.create_index(message_id_index, None).await?;
    collection.create_index(strategy_token_index, None).await?;

    Ok(())
}

pub async fn store_trade_db(
    collection: &Collection<TradeDocument>,
    trade: Trade,
    message_id: i64,
    original_message: String,
) -> Result<()> {
    let doc = match trade {
        Trade::Open(open) => TradeDocument {
            message_id,
            strategy: open.strategy,
            token: open.token,
            contract_address: open.contract_address,
            trade_type: TradeType::Open,
            original_message,
            op_type: None,
            buy_price: Some(open.buy_price),
            num_buys: Some(open.num_buys),
            total_buys: open.total_buys,
            time_window: Some(open.time_window),
            market_cap: Some(open.market_cap),
            entry_price: None,
            exit_price: None,
            profit_pct: None,
        },
        Trade::Close(close) => TradeDocument {
            message_id,
            strategy: close.strategy,
            token: close.token,
            contract_address: close.contract_address,
            trade_type: TradeType::Close,
            original_message,
            op_type: Some(close.op_type.to_string()),
            buy_price: None,
            num_buys: None,
            total_buys: None,
            time_window: None,
            market_cap: None,
            entry_price: Some(close.entry_price),
            exit_price: Some(close.exit_price),
            profit_pct: Some(close.profit_pct),
        },
    };

    collection.insert_one(doc, None).await?;
    Ok(())
}

pub async fn find_trades_by_strategy_and_token(
    collection: &Collection<TradeDocument>,
    strategy: &str,
    token: &str,
) -> Result<Vec<TradeDocument>> {
    let filter = doc! {
        "strategy": strategy,
        "token": token
    };

    let cursor = collection.find(filter, None).await?;
    let trades: Vec<TradeDocument> = cursor.try_collect().await?;

    Ok(trades)
}
