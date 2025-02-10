use anyhow::{anyhow, Result};
use serde::Serialize;
use solana_sdk::{native_token::sol_to_lamports, pubkey::Pubkey};
use std::str::FromStr;
use tracing::info;

use crate::solana::{
    dexscreener::{search_ticker, DexScreenerResponse},
    trade_raydium::{create_raydium_sol_swap_ix, create_raydium_token_swap_ix},
};

use listen_kit::solana::{
    pump::{fetch_metadata, PumpTokenInfo},
    trade_pump::{create_buy_pump_fun_ix, create_sell_pump_fun_ix},
    util::{execute_solana_transaction_with_tip, make_rpc_client},
};

pub struct MemeTrader {}

#[derive(Debug, Serialize)]
pub enum TokenInfo {
    Pump(PumpTokenInfo),
    Dexscreener(DexScreenerResponse),
}

impl MemeTrader {
    pub fn new() -> Self {
        Self {}
    }

    /// Meta buy function is all ecompasing buy function.
    /// 1. It first checks token metadata on Pump.fun API.
    ///  1.1 If the metadata are found and the bonding curve is not complete it will buy on Pump.fun.
    ///  1.2 If the metadata are found and the bonding curve is complete it will buy from Raydium.
    /// 2. If the metadata are not found on Pump.fun it will check on Dexscreener.
    /// 3. If the metadata is not found neither on Pump.fun nor on Dexscreener it will fallback to Pump.fun.
    pub async fn meta_buy(
        &self,
        token_address: &str,
        sol_amount: f64,
        slippage_bps: u16,
        tip_lamports: u64,
    ) -> Result<String> {
        let token_info = self.get_token_info(token_address).await;
        match token_info {
            Ok(TokenInfo::Pump(pump_info)) => {
                tracing::info!("Token is on Pump.fun {:#?}", pump_info);
                match self
                    .buy_pump_fun(token_address, sol_amount, slippage_bps, tip_lamports)
                    .await
                {
                    Ok(tx_sig) => Ok(tx_sig),
                    Err(e) => {
                        tracing::error!("Error buying on Pump.fun: {:#?}", e);
                        match self
                            .buy_raydium(
                                token_address,
                                pump_info.raydium_pool.as_str(),
                                sol_amount,
                                slippage_bps,
                                tip_lamports,
                            )
                            .await
                        {
                            Ok(tx_sig) => Ok(tx_sig),
                            Err(e) => {
                                tracing::error!("Error buying from Raydium: {:#?}", e);
                                Err(e)
                            }
                        }
                    }
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

    /// Meta buy function is all ecompasing buy function.
    /// 1. It first checks token metadata on Pump.fun API.
    ///  1.1 If the metadata are found and the bonding curve is not complete it will buy on Pump.fun.
    ///  1.2 If the metadata are found and the bonding curve is complete it will buy from Raydium.
    /// 2. If the metadata are not found on Pump.fun it will check on Dexscreener.
    /// 3. If the metadata is not found neither on Pump.fun nor on Dexscreener it will fallback to Pump.fun.
    pub async fn meta_sell(
        &self,
        token_address: &str,
        token_amount: u64,
        tip_lamports: u64,
    ) -> Result<String> {
        let token_info = self.get_token_info(token_address).await;
        match token_info {
            Ok(TokenInfo::Pump(pump_info)) => {
                tracing::info!("Token is on Pump.fun {:#?}", pump_info);
                match self
                    .sell_pump_fun(token_address, token_amount, tip_lamports)
                    .await
                {
                    Ok(tx_sig) => Ok(tx_sig),
                    Err(e) => {
                        tracing::error!("Error selling on Pump.fun: {:#?}", e);
                        match self
                            .sell_raydium(
                                token_address,
                                pump_info.raydium_pool.as_str(),
                                token_amount,
                                tip_lamports,
                            )
                            .await
                        {
                            Ok(tx_sig) => Ok(tx_sig),
                            Err(e) => {
                                tracing::error!("Error selling on Raydium: {:#?}", e);
                                Err(e)
                            }
                        }
                    }
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
                self.sell_pump_fun(token_address, token_amount, tip_lamports)
                    .await
            }
        }
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
            tracing::info!("Dexscreener info: {:#?}", dex_info);

            let pair = dex_info
                .pairs
                .first()
                .ok_or_else(|| anyhow!("No trading pairs found"))?;

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
            "Raydium: try buying {} SOL worth of token {} on Raydium pool {}",
            sol_amount, token_address, raydium_pool
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
}
