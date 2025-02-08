use anyhow::Result;
use copy_trade_telegram::signer::{solana::LocalSolanaSigner, SignerContext};
use copy_trade_telegram::solana::raydium::RaydiumPair;
use copy_trade_telegram::solana::util::env;
use copy_trade_telegram::trade::meme_trader::MemeTrader;
use dotenv::dotenv;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Initialize tracing with more detailed console output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_line_number(true)
        .init();

    let signer = LocalSolanaSigner::new(env("SOLANA_PRIVATE_KEY"));

    SignerContext::with_signer(Arc::new(signer), async {
        let trader = MemeTrader::new();
        let token_address = "Gj7C9aztJRsMdpfUwoBM9qUaaXjRpVCNGCwCDakvsosJ";
        // let sol_amount = 0.02;
        // let slippage_bps = 500;
        // let token_amount: u64 = 10000923892;

        // Get token info
        let info = trader.get_token_info(token_address).await?;
        println!("Token info: {:?}", info);
        if !info.is_pump_fun {
            let raydium_pair = RaydiumPair::fetch_and_deserialize(
                &Pubkey::from_str(info.raydium_pool.unwrap().as_str()).unwrap(),
            );
            println!("Raydium pair: {:?}", raydium_pair.unwrap());
        }

        // let tx_sig = if info.is_pump_fun {
        // let tx_sig = trader
        //     .buy_pump_fun(token_address, sol_amount, slippage_bps)
        //     .await?;
        // println!("Buy transaction: {}", tx_sig);

        // generate the sell transaction
        // let tx_sig = trader.sell_pump_fun(token_address, token_amount).await?;
        // println!("Sell transaction: {}", tx_sig);

        Ok(())
    })
    .await?;

    Ok(())
}
