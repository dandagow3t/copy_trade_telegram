use copy_trade_telegram::tg_copy::downloader::async_main;
use dotenv::dotenv;
use tokio::runtime;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    dotenv().ok();
    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main())
}
