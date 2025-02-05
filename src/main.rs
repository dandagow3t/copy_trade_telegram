use anyhow::Result;
use copy_trade_telegram::{
    signer::{solana::LocalSolanaSigner, SignerContext},
    solana::util::env,
    tg_copy::downloader::async_main,
};
use dotenv::dotenv;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let signer = LocalSolanaSigner::new(env("SOLANA_PRIVATE_KEY"));

    SignerContext::with_signer(Arc::new(signer), async { async_main().await }).await?;

    Ok(())
}
