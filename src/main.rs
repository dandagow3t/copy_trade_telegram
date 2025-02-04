use dotenv::dotenv;
use tg_copy::downloader::async_main;
use tokio::runtime;

pub mod common;
pub mod signer;
pub mod solana;
pub mod tg_copy;
pub mod trade;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    dotenv().ok();
    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main())
}
