use std::ops::Mul;

use anyhow::Result;
use bson::{doc, oid::ObjectId};
use mongodb::Collection;
use mongodb::IndexModel;
use serde::{Deserialize, Serialize};

use super::parse_trade::OperationType;
use super::strategy::Strategy;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActiveTrade {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub token_name: String,
    pub token_address: String,
    pub strategy_id: String,
    pub initial_holdings: u64,
    pub remaining_holdings: u64,
    pub entry_price: f64,
    pub highest_price: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ActiveTrade {
    pub fn new(
        token_name: String,
        token_address: String,
        strategy_id: String,
        initial_holdings: u64,
        entry_price: f64,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: None,
            token_name,
            token_address,
            strategy_id,
            initial_holdings,
            remaining_holdings: initial_holdings,
            entry_price,
            highest_price: entry_price,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn calculate_sell_amount(
        &self,
        profit_percentage: f64,
        op_type: OperationType,
        strategy: &Strategy,
    ) -> Option<u64> {
        // Find the appropriate take profit condition
        let sell_conditions = &strategy.sell_conditions;

        // Handle trailing stop loss
        if op_type == OperationType::TrailingStopLoss {
            // Check trailing stop loss condition
            if let Some(tsl) = &sell_conditions.trailing_stop_loss_condition {
                if profit_percentage.abs() >= (tsl.trailing_stop_loss_percentage as f64) {
                    tracing::info!("> Selected trailing stop loss: {}", tsl.description);
                    return Some(self.remaining_holdings);
                }
            }
        }

        if op_type == OperationType::StopLoss {
            // Check regular stop loss condition
            if let Some(sl) = &sell_conditions.stop_loss_condition {
                if profit_percentage.abs() >= (sl.stop_loss_percentage as f64) {
                    tracing::info!("> Selected stop loss: {}", sl.description);
                    return Some(self.remaining_holdings);
                }
            }

            return None;
        }

        // Handle take profit conditions
        if let Some(tp_conditions) = &sell_conditions.take_profit_conditions {
            for condition in tp_conditions {
                if profit_percentage >= (condition.pnl_percentage as f64) {
                    tracing::info!("> Selected take profit: {}", condition.description);
                    let percentage = condition.target_open_percentage as f64 / 100.0;
                    let percentage_to_sell = 1f64 - percentage;
                    tracing::info!("> Percentage to sell: {}%", percentage_to_sell);
                    let target_selling = (self.initial_holdings as f64)
                        .mul(percentage_to_sell)
                        .round() as u64;
                    return Some(self.remaining_holdings.min(target_selling));
                }
            }
        }

        None
    }

    pub fn update_highest_price(&mut self, current_price: f64) {
        if current_price > self.highest_price {
            self.highest_price = current_price;
            self.updated_at = chrono::Utc::now().timestamp();
        }
    }
}

pub struct ActiveTradeManager {
    collection: Collection<ActiveTrade>,
}

impl ActiveTradeManager {
    pub fn new(collection: Collection<ActiveTrade>) -> Self {
        Self { collection }
    }

    pub async fn save_trade(&self, trade: &mut ActiveTrade) -> Result<()> {
        trade.updated_at = chrono::Utc::now().timestamp();

        // Find by both token_address and strategy_id
        let filter = doc! {
            "token_address": &trade.token_address,
            "strategy_id": &trade.strategy_id
        };

        if let Some(id) = trade.id {
            self.collection
                .update_one(
                    doc! { "_id": id },
                    doc! { "$set": bson::to_document(&trade)? },
                    None,
                )
                .await?;
        } else {
            let result = self.collection.insert_one(trade.clone(), None).await?;
            trade.id = Some(result.inserted_id.as_object_id().unwrap());
        }
        Ok(())
    }

    pub async fn load_all_trades(&self) -> Result<Vec<ActiveTrade>> {
        let mut trades = Vec::new();
        let mut cursor = self.collection.find(None, None).await?;

        while cursor.advance().await? {
            trades.push(cursor.deserialize_current()?);
        }

        Ok(trades)
    }

    pub async fn remove_trade(&self, token_address: &str, strategy_id: &str) -> Result<()> {
        self.collection
            .delete_one(
                doc! {
                    "token_address": token_address,
                    "strategy_id": strategy_id
                },
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn get_trade(
        &self,
        token_address: &str,
        strategy_id: &str,
    ) -> Result<Option<ActiveTrade>> {
        self.collection
            .find_one(
                doc! {
                    "token_address": token_address,
                    "strategy_id": strategy_id
                },
                None,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn update_holdings(
        &self,
        token_address: &str,
        strategy_id: &str,
        new_holdings: u64,
    ) -> Result<()> {
        self.collection
            .update_one(
                doc! {
                    "token_address": token_address,
                    "strategy_id": strategy_id
                },
                doc! {
                    "$set": {
                        "remaining_holdings": new_holdings as i64,
                        "updated_at": chrono::Utc::now().timestamp()
                    }
                },
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn setup_indexes(&self) -> Result<()> {
        self.collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! {
                        "token_address": 1,
                        "strategy_id": 1
                    })
                    .build(),
                None,
            )
            .await?;
        Ok(())
    }
}
