use anyhow::Result;
use copy_trade_telegram::tg_copy::downloader::async_main;
use dotenv::dotenv;
use listen_kit::signer::{solana::LocalSolanaSigner, SignerContext};
use listen_kit::solana::util::env;
use std::{io, sync::Arc};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "trade-bot.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy("copy_trade_telegram=info,grammers_session=warn");

    tracing_subscriber::registry()
        .with(fmt::Layer::new().with_writer(io::stdout))
        .with(fmt::Layer::new().with_writer(non_blocking))
        .with(filter)
        .init();

    let signer = LocalSolanaSigner::new(env("SOLANA_PRIVATE_KEY"));
    SignerContext::with_signer(Arc::new(signer), async { async_main().await }).await?;

    Ok(())
}
