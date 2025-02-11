use anyhow::{anyhow, Result};
use mongodb::Collection;
use serde::Serialize;
use solana_sdk::{native_token::sol_to_lamports, pubkey::Pubkey};
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;

use crate::{
    solana::{
        dexscreener::{search_ticker, DexScreenerResponse},
        trade_raydium::{create_raydium_sol_swap_ix, create_raydium_token_swap_ix},
    },
    tg_copy::{parse_trade::OperationType, strategy::Strategy},
};

use listen_kit::{
    signer::SignerContext,
    solana::{
        balance::get_balance,
        pump::{fetch_metadata, PumpTokenInfo},
        trade_pump::{create_buy_pump_fun_ix, create_sell_pump_fun_ix},
        util::{execute_solana_transaction_with_tip, make_rpc_client},
    },
};

use crate::tg_copy::active_trade::{ActiveTrade, ActiveTradeManager};

pub struct MemeTrader {
    active_trades: Arc<ActiveTradeManager>,
}

#[derive(Debug, Serialize)]
pub enum TokenInfo {
    Pump(PumpTokenInfo),
    Dexscreener(DexScreenerResponse),
}

impl MemeTrader {
    pub fn new(collection: Collection<ActiveTrade>) -> Self {
        Self {
            active_trades: Arc::new(ActiveTradeManager::new(collection)),
        }
    }

    /// Meta buy function is all ecompasing buy function.
    pub async fn meta_buy(
        &self,
        token_address: &str,
        token_name: &str,
        strategy_id: &str,
        sol_amount: f64,
        slippage_bps: u16,
        tip_lamports: u64,
        entry_price: f64,
    ) -> Result<String> {
        let tx_sig = self
            .buy_impl(token_address, sol_amount, slippage_bps, tip_lamports)
            .await?;

        let owner = SignerContext::current().await.pubkey();

        // Get token holdings and current price after purchase
        let holdings = get_balance(
            &make_rpc_client(),
            &Pubkey::from_str(&owner)?,
            &Pubkey::from_str(token_address)?,
        )
        .await?;

        tracing::info!("Holdings: {}", holdings);

        let mut active_trade = ActiveTrade::new(
            token_name.to_string(),
            token_address.to_string(),
            strategy_id.to_string(),
            holdings.parse()?,
            entry_price,
        );

        self.active_trades.save_trade(&mut active_trade).await?;

        Ok(tx_sig)
    }

    /// Meta sell function is all ecompasing sell function.
    pub async fn meta_sell(
        &self,
        token_address: &str,
        strategy_id: &str,
        profit_percentage: f64,
        op_type: OperationType,
        strategy: &Strategy,
        tip_lamports: u64,
    ) -> Result<String> {
        let active_trade = self
            .active_trades
            .get_trade(token_address, strategy_id)
            .await?
            .ok_or_else(|| anyhow!("No active trade found for token and strategy"))?;

        tracing::info!("Active trade: {:?}", active_trade);

        let sell_amount =
            match active_trade.calculate_sell_amount(profit_percentage, op_type, strategy) {
                Some(amount) => amount,
                None => {
                    tracing::info!(
                        "No sell amount could be calculated, using remaining holdings of {}",
                        active_trade.remaining_holdings
                    );
                    active_trade.remaining_holdings
                }
            };

        tracing::info!("Sell amount: {:?}", sell_amount);

        let tx_sig = self
            .sell_impl(token_address, sell_amount, tip_lamports)
            .await?;

        // Update or remove the trade based on remaining holdings
        let new_holdings = active_trade.remaining_holdings - sell_amount;
        if new_holdings == 0 {
            self.active_trades
                .remove_trade(token_address, strategy_id)
                .await?;
        } else {
            self.active_trades
                .update_holdings(token_address, strategy_id, new_holdings)
                .await?;
        }

        Ok(tx_sig)
    }

    /// Get information about a meme token from either Pump.fun or Dexscreener
    pub async fn get_token_info(&self, token_address: &str) -> Result<TokenInfo> {
        // Try Pump.fun first
        let pump_result = match Pubkey::from_str(token_address) {
            Ok(mint) => fetch_metadata(&mint).await,
            Err(_) => Err(anyhow!("Invalid Solana address format")),
        };

        // If Pump.fun fails, try Dexscreener
        if pump_result.is_err() {
            let dex_info = search_ticker(token_address.to_string()).await?;
            let pairs = dex_info
                .pairs
                .first()
                .ok_or_else(|| anyhow!("No trading pairs found"))?;
            tracing::info!("Dexscreener pairs: {:?}", pairs);
            Ok(TokenInfo::Dexscreener(dex_info))
        } else {
            Ok(TokenInfo::Pump(pump_result.unwrap()))
        }
    }

    /// Buy a token on Pump.fun
    pub async fn buy_pump_fun(
        &self,
        token_address: &str,
        sol_amount: f64,
        slippage_bps: u16,
        tip_lamports: u64,
    ) -> Result<String> {
        info!(
            "Pump.fun: try buying {} SOL worth of token {}",
            sol_amount, token_address
        );
        let token_address = token_address.to_string();

        execute_solana_transaction_with_tip(
            move |owner| async move {
                create_buy_pump_fun_ix(
                    token_address.to_string(),
                    sol_to_lamports(sol_amount),
                    slippage_bps,
                    &make_rpc_client(),
                    &owner,
                )
                .await
            },
            tip_lamports,
        )
        .await
    }

    /// Sell a token on Pump.fun
    pub async fn sell_pump_fun(
        &self,
        token_address: &str,
        token_amount: u64,
        tip_lamports: u64,
    ) -> Result<String> {
        info!("Selling {} tokens of {}", token_amount, token_address);

        let token_address = token_address.to_string();
        execute_solana_transaction_with_tip(
            move |owner| async move {
                create_sell_pump_fun_ix(token_address.to_string(), token_amount, &owner).await
            },
            tip_lamports,
        )
        .await
    }

    pub async fn buy_raydium(
        &self,
        token_address: &str,
        raydium_pool: &str,
        sol_amount: f64,
        slippage_bps: u16,
        tip_lamports: u64,
    ) -> Result<String> {
        info!(
            "Raydium: try buying {} SOL worth of token {}",
            sol_amount, token_address
        );
        let raydium_pool = raydium_pool.to_string();
        let token_address = token_address.to_string();

        execute_solana_transaction_with_tip(
            move |owner| async move {
                create_raydium_sol_swap_ix(
                    raydium_pool,
                    sol_to_lamports(sol_amount),
                    slippage_bps,
                    Pubkey::from_str(token_address.as_str())?,
                    &make_rpc_client(),
                    &owner,
                )
                .await
            },
            tip_lamports,
        )
        .await
    }

    pub async fn sell_raydium(
        &self,
        token_address: &str,
        raydium_pool: &str,
        token_amount: u64,
        tip_lamports: u64,
    ) -> Result<String> {
        info!(
            "Raydium: try selling {} tokens of {} on Raydium pool {}",
            token_amount, token_address, raydium_pool
        );
        let raydium_pool = raydium_pool.to_string();
        let token_address = token_address.to_string();

        execute_solana_transaction_with_tip(
            move |owner| async move {
                create_raydium_token_swap_ix(
                    raydium_pool,
                    token_amount as u64,
                    Pubkey::from_str(token_address.as_str())?, // Token
                    &make_rpc_client(),
                    &owner,
                )
                .await
            },
            tip_lamports,
        )
        .await
    }

    /// Internal buy implementation that handles the actual trading logic
    async fn buy_impl(
        &self,
        token_address: &str,
        sol_amount: f64,
        slippage_bps: u16,
        tip_lamports: u64,
    ) -> Result<String> {
        let token_info = self.get_token_info(token_address).await;

        match token_info {
            Ok(TokenInfo::Pump(pump_info)) => {
                match pump_info.complete {
                    true => tracing::info!(
                        "Pump.fun: complete, buying from Raydium; pool {}",
                        pump_info.raydium_pool
                    ),
                    false => tracing::info!(
                        "Pump.fun: incomplete, bonding curve {}",
                        pump_info.bonding_curve
                    ),
                }

                if !pump_info.complete {
                    self.buy_pump_fun(token_address, sol_amount, slippage_bps, tip_lamports)
                        .await
                } else {
                    self.buy_raydium(
                        token_address,
                        pump_info.raydium_pool.as_str(),
                        sol_amount,
                        slippage_bps,
                        tip_lamports,
                    )
                    .await
                }
            }

            Ok(TokenInfo::Dexscreener(dex_info)) => {
                tracing::info!("Token is on Dexscreener {:#?}", dex_info);
                // self.buy_dexscreener(token_address, sol_amount, slippage_bps)
                //     .await
                Ok(String::new())
            }
            _ => {
                tracing::info!(
                    "Token info not found on Pump.fun or Dexscreener. Fallback to Pump.fun"
                );
                self.buy_pump_fun(token_address, sol_amount, slippage_bps, tip_lamports)
                    .await
            }
        }
    }

    /// Internal sell implementation that handles the actual trading logic
    async fn sell_impl(
        &self,
        token_address: &str,
        token_amount: u64,
        tip_lamports: u64,
    ) -> Result<String> {
        let token_info = self.get_token_info(token_address).await;

        match token_info {
            Ok(TokenInfo::Pump(pump_info)) => {
                match pump_info.complete {
                    true => tracing::info!(
                        "Pump.fun: complete, selling on Raydium; pool {}",
                        pump_info.raydium_pool
                    ),
                    false => tracing::info!(
                        "Pump.fun: incomplete, selling on bonding curve {}",
                        pump_info.bonding_curve
                    ),
                }

                if !pump_info.complete {
                    self.sell_pump_fun(token_address, token_amount, tip_lamports)
                        .await
                } else {
                    self.sell_raydium(
                        token_address,
                        pump_info.raydium_pool.as_str(),
                        token_amount,
                        tip_lamports,
                    )
                    .await
                }
            }
            Ok(TokenInfo::Dexscreener(dex_info)) => {
                tracing::info!("Token is on Dexscreener {:#?}", dex_info);
                // For now, we'll just return an error since Dexscreener selling is not implemented
                Err(anyhow!("Selling on Dexscreener not implemented yet"))
            }
            _ => {
                tracing::info!(
                    "Token info not found on Pump.fun or Dexscreener. Fallback to Pump.fun"
                );
                self.sell_pump_fun(token_address, token_amount, tip_lamports)
                    .await
            }
        }
    }
}
