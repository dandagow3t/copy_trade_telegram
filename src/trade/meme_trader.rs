use anyhow::{anyhow, Result};
use solana_sdk::{native_token::sol_to_lamports, pubkey::Pubkey};
use std::str::FromStr;
use tracing::{info, warn};

use crate::solana::{
    dexscreener::search_ticker,
    pump::{fetch_metadata, get_bonding_curve, mint_to_pump_accounts},
    trade_pump::{create_buy_pump_fun_ix, create_sell_pump_fun_ix},
    util::{execute_solana_transaction_with_priority, make_rpc_client},
};

pub struct MemeTrader {
    rpc_client: solana_client::nonblocking::rpc_client::RpcClient,
}

#[derive(Debug)]
pub struct MemeTokenInfo {
    pub name: String,
    pub symbol: String,
    pub price: f64,
    pub market_cap: f64,
    pub volume_24h: f64,
    pub is_pump_fun: bool,
    pub dex: String,
    pub raydium_pool: Option<String>,
}

impl MemeTrader {
    pub fn new() -> Self {
        Self {
            rpc_client: make_rpc_client(),
        }
    }

    /// Get information about a meme token from either Pump.fun or Dexscreener
    pub async fn get_token_info(&self, token_address: &str) -> Result<MemeTokenInfo> {
        // Try Pump.fun first
        match Pubkey::from_str(token_address) {
            Ok(mint) => {
                if let Ok(metadata) = fetch_metadata(&mint).await {
                    return Ok(MemeTokenInfo {
                        name: metadata.name,
                        symbol: metadata.symbol,
                        price: metadata.usd_market_cap / metadata.total_supply as f64,
                        market_cap: metadata.usd_market_cap,
                        volume_24h: 0.0, // Pump.fun doesn't provide 24h volume
                        is_pump_fun: true,
                        dex: "pump.fun".to_string(),
                        raydium_pool: None,
                    });
                }
            }
            Err(_) => warn!("Invalid Solana address format"),
        }

        // Fallback to Dexscreener
        let dex_info = search_ticker(token_address.to_string()).await?;
        let pair = dex_info
            .pairs
            .first()
            .ok_or_else(|| anyhow!("No trading pairs found"))?;

        Ok(MemeTokenInfo {
            name: pair.base_token.name.clone(),
            symbol: pair.base_token.symbol.clone(),
            price: pair.price_usd.parse::<f64>().unwrap_or(0.0),
            market_cap: pair.liquidity.usd,
            volume_24h: pair.volume.h24,
            is_pump_fun: false,
            dex: pair.dex_id.clone(),
            raydium_pool: if pair.dex_id == "raydium" {
                Some(pair.pair_address.clone())
            } else {
                None
            },
        })
    }

    /// Buy a token on Pump.fun
    pub async fn buy_pump_fun(
        &self,
        token_address: &str,
        sol_amount: f64,
        slippage_bps: u16,
    ) -> Result<String> {
        info!("Buying {} SOL worth of token {}", sol_amount, token_address);

        // Verify it's a Pump.fun token first
        let mint = Pubkey::from_str(token_address)?;
        let pump_accounts = mint_to_pump_accounts(&mint);

        // Get bonding curve to verify token exists
        let bonding_curve =
            get_bonding_curve(&self.rpc_client, pump_accounts.bonding_curve).await?;

        if bonding_curve.complete {
            return Err(anyhow!("Token is already completed/rugged"));
        }

        let token_address = token_address.to_string();

        execute_solana_transaction_with_priority(move |owner| async move {
            create_buy_pump_fun_ix(
                token_address.to_string(),
                sol_to_lamports(sol_amount),
                slippage_bps,
                &make_rpc_client(),
                &owner,
            )
            .await
        })
        .await
    }

    /// Sell a token on Pump.fun
    pub async fn sell_pump_fun(&self, token_address: &str, token_amount: u64) -> Result<String> {
        info!("Selling {} tokens of {}", token_amount, token_address);

        let token_address = token_address.to_string();
        execute_solana_transaction_with_priority(move |owner| async move {
            create_sell_pump_fun_ix(token_address.to_string(), token_amount, &owner).await
        })
        .await
    }

    // pub async fn buy_on_jupiter(
    //     &self,
    //     token_address: &str,
    //     sol_amount: f64,
    //     slippage_bps: u16,
    // ) -> Result<String> {
    //     info!(
    //         "Buying {} SOL worth of token {} on Jupiter",
    //         sol_amount, token_address
    //     );

    //     let token_address = token_address.to_string();
    //     execute_solana_transactions(move |owner| async move {
    //         create_trade_transaction(
    //             "So11111111111111111111111111111111111111112".to_string(), // WSOL
    //             sol_to_lamports(sol_amount),
    //             token_address.to_string(),
    //             slippage_bps,
    //             &owner,
    //         )
    //         .await
    //     })
    //     .await
    // }
    // // TODO: Add Raydium trading functions when needed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_token_info_pump_fun() {
        let trader = MemeTrader::new();
        let info = trader
            .get_token_info("4cRkQ2dntpusYag6Zmvco8T78WxK9Jqh1eEZJox8pump")
            .await
            .unwrap();

        assert!(info.is_pump_fun);
        assert_eq!(info.symbol, "🗿");
    }

    #[tokio::test]
    async fn test_get_token_info_raydium() {
        let trader = MemeTrader::new();
        let info = trader
            .get_token_info("BONK97G5KsjgR9jFbwcsUhoSgwKBQ9pRY1g6gQpKEanB")
            .await
            .unwrap();

        assert!(!info.is_pump_fun);
        assert_eq!(info.symbol, "BONK");
    }

    #[tokio::test]
    #[ignore] // Requires wallet with SOL
    async fn test_buy_pump_fun() {
        let trader = MemeTrader::new();
        let result = trader
            .buy_pump_fun(
                "4cRkQ2dntpusYag6Zmvco8T78WxK9Jqh1eEZJox8pump",
                0.01, // 0.01 SOL
                500,  // 5% slippage
            )
            .await;
        assert!(result.is_ok());
    }
}
