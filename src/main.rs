use anyhow::Result;
use copy_trade_telegram::{
    signer::{solana::LocalSolanaSigner, SignerContext},
    solana::util::env,
    tg_copy::downloader::async_main,
};
use dotenv::dotenv;
use std::sync::Arc;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Configure logging
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy("copy_trade_telegram=info,grammers_session=warn");

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let signer = LocalSolanaSigner::new(env("SOLANA_PRIVATE_KEY"));

    SignerContext::with_signer(Arc::new(signer), async { async_main().await }).await?;

    Ok(())
}
