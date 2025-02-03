use dotenv::dotenv;
use downloader::async_main;
use tokio::runtime;

mod db;
mod downloader;
mod parse_trade;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    dotenv().ok();
    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main())
}
