use copy_trade_telegram::trade::meme_trader::MemeTrader;

#[tokio::main]
async fn main() -> Result<()> {
    let trader = MemeTrader::new();

    // Get token info
    let info = trader
        .get_token_info("4cRkQ2dntpusYag6Zmvco8T78WxK9Jqh1eEZJox8pump")
        .await?;
    println!("Token info: {:?}", info);

    // Buy token
    if info.is_pump_fun {
        let tx_sig = trader
            .buy_pump_fun(
                "4cRkQ2dntpusYag6Zmvco8T78WxK9Jqh1eEZJox8pump",
                0.1, // 0.1 SOL
                500, // 5% slippage
            )
            .await?;
        println!("Buy transaction: {}", tx_sig);
    }

    Ok(())
}
