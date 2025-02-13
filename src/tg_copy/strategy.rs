use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Strategy {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    #[serde(rename = "strategyId")]
    pub strategy_id: String,
    #[serde(rename = "isShaved")]
    pub is_shaved: bool,
    #[serde(rename = "buyConditions")]
    pub buy_conditions: Vec<BuyCondition>,
    #[serde(rename = "sellConditions")]
    pub sell_conditions: SellConditions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuyCondition {
    #[serde(rename = "timeWindowSeconds")]
    pub time_window_seconds: i32,
    #[serde(rename = "minSolBuyDelta")]
    pub min_sol_buy_delta: f32,
    #[serde(rename = "minWallets")]
    pub min_wallets: i32,
    #[serde(rename = "minMarketcap")]
    pub min_marketcap: u64,
    #[serde(rename = "maxMarketcap")]
    pub max_marketcap: Option<u64>,
    #[serde(rename = "solBuyAmount")]
    pub sol_buy_amount: f32,
    #[serde(rename = "top10MaxPercentage")]
    pub top10_max_percentage: f32,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SellConditions {
    #[serde(rename = "takeProfitConditions")]
    pub take_profit_conditions: Option<Vec<TakeProfitCondition>>,
    #[serde(rename = "stopLossCondition")]
    pub stop_loss_condition: Option<StopLossCondition>,
    #[serde(rename = "trailingStopLossCondition")]
    pub trailing_stop_loss_condition: Option<TrailingStopLossCondition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TakeProfitCondition {
    #[serde(rename = "pnlPercentage")]
    pub pnl_percentage: i32,
    #[serde(rename = "targetOpenPercentage")]
    pub target_open_percentage: i32,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StopLossCondition {
    #[serde(rename = "stopLossPercentage")]
    pub stop_loss_percentage: i32,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrailingStopLossCondition {
    #[serde(rename = "trailingStopLossPercentage")]
    pub trailing_stop_loss_percentage: f32,
    #[serde(rename = "isLogarithmic")]
    pub is_logarithmic: bool,
    pub description: String,
}
