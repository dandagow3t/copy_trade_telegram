use borsh::{BorshDeserialize, BorshSerialize};
use log::{debug, error, warn};
use serde::Serialize;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};

use anyhow::{anyhow, Result};
use solana_sdk::instruction::{AccountMeta, Instruction};
use std::str::FromStr;
use tokio::time::{sleep, Duration};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RaydiumPoolLayout {
    pub status: u64,
    pub nonce: u64,
    pub max_order: u64,
    pub depth: u64,
    pub base_decimal: u64,
    pub quote_decimal: u64,
    pub state: u64,
    pub reset_flag: u64,
    pub min_size: u64,
    pub vol_max_cut_ratio: u64,
    pub amount_wave_ratio: u64,
    pub base_lot_size: u64,
    pub quote_lot_size: u64,
    pub min_price_multiplier: u64,
    pub max_price_multiplier: u64,
    pub system_decimal_value: u64,
    pub min_separate_numerator: u64,
    pub min_separate_denominator: u64,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub pnl_numerator: u64,
    pub pnl_denominator: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
    pub base_need_take_pnl: u64,
    pub quote_need_take_pnl: u64,
    pub quote_total_pnl: u64,
    pub base_total_pnl: u64,
    pub pool_open_time: u64,
    pub punish_pc_amount: u64,
    pub punish_coin_amount: u64,
    pub orderbook_to_init_time: u64,
    pub swap_base_in_amount: u128,
    pub swap_quote_out_amount: u128,
    pub swap_base2_quote_fee: u64,
    pub swap_quote_in_amount: u128,
    pub swap_base_out_amount: u128,
    pub swap_quote2_base_fee: u64,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market_id: Pubkey,
    pub market_program_id: Pubkey,
    pub target_orders: Pubkey,
    pub withdraw_queue: Pubkey,
    pub lp_vault: Pubkey,
    pub owner: Pubkey,
    pub lp_reserve: u64,
    pub padding: [u64; 3],
}

impl RaydiumPoolLayout {
    pub const LEN: usize = 8 + // status
        8 + // nonce
        8 + // max_order
        8 + // depth
        8 + // base_decimal
        8 + // quote_decimal
        8 + // state
        8 + // reset_flag
        8 + // min_size
        8 + // vol_max_cut_ratio
        8 + // amount_wave_ratio
        8 + // base_lot_size
        8 + // quote_lot_size
        8 + // min_price_multiplier
        8 + // max_price_multiplier
        8 + // system_decimal_value
        8 + // min_separate_numerator
        8 + // min_separate_denominator
        8 + // trade_fee_numerator
        8 + // trade_fee_denominator
        8 + // pnl_numerator
        8 + // pnl_denominator
        8 + // swap_fee_numerator
        8 + // swap_fee_denominator
        8 + // base_need_take_pnl
        8 + // quote_need_take_pnl
        8 + // quote_total_pnl
        8 + // base_total_pnl
        8 + // pool_open_time
        8 + // punish_pc_amount
        8 + // punish_coin_amount
        8 + // orderbook_to_init_time
        16 + // swap_base_in_amount (u128)
        16 + // swap_quote_out_amount (u128)
        8 + // swap_base2_quote_fee
        16 + // swap_quote_in_amount (u128)
        16 + // swap_base_out_amount (u128)
        8 + // swap_quote2_base_fee
        32 + // base_vault
        32 + // quote_vault
        32 + // base_mint
        32 + // quote_mint
        32 + // lp_mint
        32 + // open_orders
        32 + // market_id
        32 + // market_program_id
        32 + // target_orders
        32 + // withdraw_queue
        32 + // lp_vault
        32 + // owner
        8 + // lp_reserve
        24; // padding ([u64; 3])

    pub fn parse(data: &[u8]) -> Result<Self, std::io::Error> {
        Self::try_from_slice(data)
    }
}

/// Raydium swap instruction accounts
#[derive(Debug, Serialize)]
pub struct RaydiumAccounts {
    pub amm: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub pool_coin_token_account: Pubkey,
    pub pool_pc_token_account: Pubkey,
    pub serum_market: Pubkey,
}

pub async fn get_raydium_pool(
    rpc_client: &RpcClient,
    raydium_pool_pubkey: &Pubkey,
) -> Result<RaydiumPoolLayout> {
    const MAX_RETRIES: u32 = 5;
    const INITIAL_DELAY_MS: u64 = 200;
    let mut retries = 0;
    let mut delay = Duration::from_millis(INITIAL_DELAY_MS);

    loop {
        match rpc_client
            .get_account_with_config(
                &raydium_pool_pubkey,
                RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(CommitmentConfig::processed()),
                    data_slice: None,
                    min_context_slot: None,
                },
            )
            .await
        {
            Ok(res) => {
                if let Some(account) = res.value {
                    // Convert Vec<u8> to [u8; 49]
                    let data_length = account.data.len();
                    tracing::info!(
                        "Data length vs expected: {:?}/{:?}",
                        data_length,
                        RaydiumPoolLayout::LEN
                    );
                    let data: [u8; RaydiumPoolLayout::LEN] = account
                        .data
                        .try_into()
                        .map_err(|_| anyhow!("Invalid data length: {}", data_length))?;

                    debug!("Raw bytes: {:?}", data);

                    let layout = RaydiumPoolLayout {
                        status: u64::from_le_bytes(data[0..8].try_into()?),
                        nonce: u64::from_le_bytes(data[8..16].try_into()?),
                        max_order: u64::from_le_bytes(data[16..24].try_into()?),
                        depth: u64::from_le_bytes(data[24..32].try_into()?),
                        base_decimal: u64::from_le_bytes(data[32..40].try_into()?),
                        quote_decimal: u64::from_le_bytes(data[40..48].try_into()?),
                        state: u64::from_le_bytes(data[48..56].try_into()?),
                        reset_flag: u64::from_le_bytes(data[56..64].try_into()?),
                        min_size: u64::from_le_bytes(data[64..72].try_into()?),
                        vol_max_cut_ratio: u64::from_le_bytes(data[72..80].try_into()?),
                        amount_wave_ratio: u64::from_le_bytes(data[80..88].try_into()?),
                        base_lot_size: u64::from_le_bytes(data[88..96].try_into()?),
                        quote_lot_size: u64::from_le_bytes(data[96..104].try_into()?),
                        min_price_multiplier: u64::from_le_bytes(data[104..112].try_into()?),
                        max_price_multiplier: u64::from_le_bytes(data[112..120].try_into()?),
                        system_decimal_value: u64::from_le_bytes(data[120..128].try_into()?),
                        min_separate_numerator: u64::from_le_bytes(data[128..136].try_into()?),
                        min_separate_denominator: u64::from_le_bytes(data[136..144].try_into()?),
                        trade_fee_numerator: u64::from_le_bytes(data[144..152].try_into()?),
                        trade_fee_denominator: u64::from_le_bytes(data[152..160].try_into()?),
                        pnl_numerator: u64::from_le_bytes(data[160..168].try_into()?),
                        pnl_denominator: u64::from_le_bytes(data[168..176].try_into()?),
                        swap_fee_numerator: u64::from_le_bytes(data[176..184].try_into()?),
                        swap_fee_denominator: u64::from_le_bytes(data[184..192].try_into()?),
                        base_need_take_pnl: u64::from_le_bytes(data[192..200].try_into()?),
                        quote_need_take_pnl: u64::from_le_bytes(data[200..208].try_into()?),
                        quote_total_pnl: u64::from_le_bytes(data[208..216].try_into()?),
                        base_total_pnl: u64::from_le_bytes(data[216..224].try_into()?),
                        pool_open_time: u64::from_le_bytes(data[224..232].try_into()?),
                        punish_pc_amount: u64::from_le_bytes(data[232..240].try_into()?),
                        punish_coin_amount: u64::from_le_bytes(data[240..248].try_into()?),
                        orderbook_to_init_time: u64::from_le_bytes(data[248..256].try_into()?),
                        swap_base_in_amount: u128::from_le_bytes(data[256..272].try_into()?),
                        swap_quote_out_amount: u128::from_le_bytes(data[272..288].try_into()?),
                        swap_base2_quote_fee: u64::from_le_bytes(data[288..296].try_into()?),
                        swap_quote_in_amount: u128::from_le_bytes(data[296..312].try_into()?),
                        swap_base_out_amount: u128::from_le_bytes(data[312..328].try_into()?),
                        swap_quote2_base_fee: u64::from_le_bytes(data[328..336].try_into()?),
                        base_vault: Pubkey::try_from_slice(&data[336..368])?,
                        quote_vault: Pubkey::try_from_slice(&data[368..400])?, // Fixed: was 368..392
                        base_mint: Pubkey::try_from_slice(&data[400..432])?, // Fixed: adjusted subsequent ranges
                        quote_mint: Pubkey::try_from_slice(&data[432..464])?,
                        lp_mint: Pubkey::try_from_slice(&data[464..496])?,
                        open_orders: Pubkey::try_from_slice(&data[496..528])?,
                        market_id: Pubkey::try_from_slice(&data[528..560])?,
                        market_program_id: Pubkey::try_from_slice(&data[560..592])?,
                        target_orders: Pubkey::try_from_slice(&data[592..624])?,
                        withdraw_queue: Pubkey::try_from_slice(&data[624..656])?,
                        lp_vault: Pubkey::try_from_slice(&data[656..688])?,
                        owner: Pubkey::try_from_slice(&data[688..720])?,
                        lp_reserve: u64::from_le_bytes(data[720..728].try_into()?),
                        padding: [
                            u64::from_le_bytes(data[728..736].try_into()?),
                            u64::from_le_bytes(data[736..744].try_into()?),
                            u64::from_le_bytes(data[744..752].try_into()?),
                        ],
                    };

                    debug!("Parsed RaydiumPairLayout: {:?}", layout);
                    return Ok(layout);
                } else {
                    if retries >= MAX_RETRIES {
                        error!("Max retries reached. Account not found.");
                        return Err(anyhow!("Account not found after max retries"));
                    }
                    warn!(
                        "Attempt {} failed: Account not found. Retrying in {:?}...",
                        retries + 1,
                        delay
                    );
                    sleep(delay).await;
                    retries += 1;
                    delay = Duration::from_millis(INITIAL_DELAY_MS * 2u64.pow(retries));
                    continue;
                }
            }
            Err(e) => {
                if retries >= MAX_RETRIES {
                    error!("Max retries reached. Last error: {}", e);
                    return Err(anyhow!("Max retries reached. Last error: {}", e));
                }
                warn!(
                    "Attempt {} failed: {}. Retrying in {:?}...",
                    retries + 1,
                    e,
                    delay
                );
                sleep(delay).await;
                retries += 1;
                delay = Duration::from_millis(INITIAL_DELAY_MS * 2u64.pow(retries));
            }
        }
    }
}

pub async fn get_raydium_accounts(
    rpc_client: &RpcClient,
    raydium_pool_pubkey: Pubkey,
) -> Result<RaydiumAccounts> {
    match get_raydium_pool(rpc_client, &raydium_pool_pubkey).await {
        Ok(pool) => Ok(RaydiumAccounts {
            amm: raydium_pool_pubkey,
            amm_open_orders: pool.open_orders,
            amm_target_orders: pool.target_orders,
            pool_coin_token_account: pool.base_vault,
            pool_pc_token_account: pool.quote_vault,
            serum_market: pool.market_id,
        }),
        Err(e) => Err(e),
    }
}

pub fn extract_raydium_accounts(
    raydium_pool_pubkey: Pubkey,
    pool: &RaydiumPoolLayout,
) -> RaydiumAccounts {
    RaydiumAccounts {
        amm: raydium_pool_pubkey,
        amm_open_orders: pool.open_orders,
        amm_target_orders: pool.target_orders,
        pool_coin_token_account: pool.base_vault,
        pool_pc_token_account: pool.quote_vault,
        serum_market: pool.market_id,
    }
}

#[derive(Debug)]
pub struct SerumAccounts {
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_queue: Pubkey,
    pub coin_vault_account: Pubkey,
    pub pc_vault_account: Pubkey,
    pub vault_signer: Pubkey,
}

pub const RAYDIUM_V4_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const RAYDIUM_V4_AUTHORITY: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
pub const SERUM_PROGRAM: &str = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX";
pub const RAYDIUM_V4_BUY_METHOD: u8 = 9;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct SerumMarketLayout {
    pub blob_5: [u8; 5],
    pub account_flags: [u8; 8],
    pub own_address: Pubkey,
    pub vault_signer_nonce: u64,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub base_deposits_total: u64,
    pub base_fees_accrued: u64,
    pub quote_vault: Pubkey,
    pub quote_deposits_total: u64,
    pub quote_fees_accrued: u64,
    pub quote_dust_threshold: u64,
    pub request_queue: Pubkey,
    pub event_queue: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub base_lot_size: u64,
    pub quote_lot_size: u64,
    pub fee_rate_bps: u64,
    pub referrer_rebates_accrued: u64,
    pub blob_7: [u8; 7],
}

impl SerumMarketLayout {
    pub const LEN: usize = 5 + // blob_5
        8 + // account_flags
        32 + // own_address
        8 + // vault_signer_nonce
        32 + // base_mint
        32 + // quote_mint
        32 + // base_vault
        8 + // base_deposits_total
        8 + // base_fees_accrued
        32 + // quote_vault
        8 + // quote_deposits_total
        8 + // quote_fees_accrued
        8 + // quote_dust_threshold
        32 + // request_queue
        32 + // event_queue
        32 + // bids
        32 + // asks
        8 + // base_lot_size
        8 + // quote_lot_size
        8 + // fee_rate_bps
        8 + // referrer_rebates_accrued
        7; // blob_7

    pub fn parse(data: &[u8]) -> Result<Self, std::io::Error> {
        Self::try_from_slice(data)
    }
}

pub async fn get_serum_market(
    rpc_client: &RpcClient,
    market_pubkey: Pubkey,
) -> Result<SerumMarketLayout> {
    const MAX_RETRIES: u32 = 5;
    const INITIAL_DELAY_MS: u64 = 200;
    let mut retries = 0;
    let mut delay = Duration::from_millis(INITIAL_DELAY_MS);

    loop {
        match rpc_client
            .get_account_with_config(
                &market_pubkey,
                RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(CommitmentConfig::processed()),
                    data_slice: None,
                    min_context_slot: None,
                },
            )
            .await
        {
            Ok(res) => {
                if let Some(account) = res.value {
                    let data_length = account.data.len();
                    tracing::info!(
                        "Data length vs expected: {:?}/{:?}",
                        data_length,
                        SerumMarketLayout::LEN
                    );

                    let data: [u8; SerumMarketLayout::LEN] = account
                        .data
                        .try_into()
                        .map_err(|_| anyhow!("Invalid data length: {}", data_length))?;

                    let mut offset = 0;
                    let blob_5 = data[offset..offset + 5].try_into()?;
                    offset += 5;
                    let account_flags = data[offset..offset + 8].try_into()?;
                    offset += 8;
                    let own_address = Pubkey::try_from_slice(&data[offset..offset + 32])?;
                    offset += 32;
                    let vault_signer_nonce =
                        u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let base_mint = Pubkey::try_from_slice(&data[offset..offset + 32])?;
                    offset += 32;
                    let quote_mint = Pubkey::try_from_slice(&data[offset..offset + 32])?;
                    offset += 32;
                    let base_vault = Pubkey::try_from_slice(&data[offset..offset + 32])?;
                    offset += 32;
                    let base_deposits_total =
                        u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let base_fees_accrued =
                        u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let quote_vault = Pubkey::try_from_slice(&data[offset..offset + 32])?;
                    offset += 32;
                    let quote_deposits_total =
                        u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let quote_fees_accrued =
                        u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let quote_dust_threshold =
                        u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let request_queue = Pubkey::try_from_slice(&data[offset..offset + 32])?;
                    offset += 32;
                    let event_queue = Pubkey::try_from_slice(&data[offset..offset + 32])?;
                    offset += 32;
                    let bids = Pubkey::try_from_slice(&data[offset..offset + 32])?;
                    offset += 32;
                    let asks = Pubkey::try_from_slice(&data[offset..offset + 32])?;
                    offset += 32;
                    let base_lot_size = u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let quote_lot_size = u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let fee_rate_bps = u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let referrer_rebates_accrued =
                        u64::from_le_bytes(data[offset..offset + 8].try_into()?);
                    offset += 8;
                    let blob_7 = data[offset..offset + 7].try_into()?;

                    let layout = SerumMarketLayout {
                        blob_5,
                        account_flags,
                        own_address,
                        vault_signer_nonce,
                        base_mint,
                        quote_mint,
                        base_vault,
                        base_deposits_total,
                        base_fees_accrued,
                        quote_vault,
                        quote_deposits_total,
                        quote_fees_accrued,
                        quote_dust_threshold,
                        request_queue,
                        event_queue,
                        bids,
                        asks,
                        base_lot_size,
                        quote_lot_size,
                        fee_rate_bps,
                        referrer_rebates_accrued,
                        blob_7,
                    };

                    debug!("Parsed SerumMarketLayout: {:?}", layout);
                    return Ok(layout);
                } else {
                    if retries >= MAX_RETRIES {
                        error!("Max retries reached. Account not found.");
                        return Err(anyhow!("Account not found after max retries"));
                    }
                    warn!(
                        "Attempt {} failed: Account not found. Retrying in {:?}...",
                        retries + 1,
                        delay
                    );
                    sleep(delay).await;
                    retries += 1;
                    delay = Duration::from_millis(INITIAL_DELAY_MS * 2u64.pow(retries));
                    continue;
                }
            }
            Err(e) => {
                if retries >= MAX_RETRIES {
                    error!("Max retries reached. Last error: {}", e);
                    return Err(anyhow!("Max retries reached. Last error: {}", e));
                }
                warn!(
                    "Attempt {} failed: {}. Retrying in {:?}...",
                    retries + 1,
                    e,
                    delay
                );
                sleep(delay).await;
                retries += 1;
                delay = Duration::from_millis(INITIAL_DELAY_MS * 2u64.pow(retries));
            }
        }
    }
}

pub async fn get_serum_accounts(
    rpc_client: &RpcClient,
    serum_market_pubkey: Pubkey,
) -> Result<SerumAccounts> {
    match get_serum_market(rpc_client, serum_market_pubkey).await {
        Ok(market) => {
            let vault_signer = Pubkey::create_program_address(
                &[
                    serum_market_pubkey.as_ref(),
                    &market.vault_signer_nonce.to_le_bytes(),
                ],
                &Pubkey::from_str(SERUM_PROGRAM)?,
            )
            .map_err(|e| anyhow!("Failed to create program address: {}", e))?;

            Ok(SerumAccounts {
                bids: market.bids,
                asks: market.asks,
                event_queue: market.event_queue,
                coin_vault_account: market.base_vault,
                pc_vault_account: market.quote_vault,
                vault_signer,
            })
        }
        Err(e) => Err(e),
    }
}

pub fn calculate_minimum_amount_out(
    pool_state: &RaydiumPoolLayout,
    amount_in: u64,
    slippage_tolerance: f64, // e.g., 0.01 for 1%
) -> u64 {
    // First get the swap fee
    let fee_numerator = pool_state.swap_fee_numerator;
    let fee_denominator = pool_state.swap_fee_denominator;

    // Get current pool ratios
    let base_amount = pool_state.swap_base_in_amount;
    let quote_amount = pool_state.swap_quote_out_amount;

    // Calculate the swap fee
    let fee_amount = amount_in
        .checked_mul(fee_numerator)
        .unwrap()
        .checked_div(fee_denominator)
        .unwrap();

    // Amount after fee
    let amount_in_after_fees = amount_in.checked_sub(fee_amount).unwrap();

    // Calculate expected output using constant product formula (x * y = k)
    let amount_out = quote_amount
        .checked_mul(amount_in_after_fees as u128)
        .unwrap()
        .checked_div(
            base_amount
                .checked_add(amount_in_after_fees as u128)
                .unwrap(),
        )
        .unwrap();

    // Apply slippage tolerance
    let min_amount_out = (amount_out as f64 * (1.0 - slippage_tolerance)) as u64;

    min_amount_out
}

#[derive(BorshSerialize)]
struct SwapInstructionData {
    // Single byte discriminator for swap
    instruction: u8, // Value: 9
    amount_in: u64,
    minimum_amount_out: u64,
}

/// Interact With Raydium Liquidity Pool V4 (675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8)
/// Input Accounts
/// #1 - Token Program: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA (Program)
/// #2 - Amm: Raydium (SOL-RETAIL) Market (Writable)
/// #3 - Amm Authority: Raydium Authority V4
/// #4 - Amm Open Orders: 5C7RNQnRwt8ardd7Qa98HHf6iMX9WGzr2o9aJWdR14yy (Writable)
/// #5 - Amm Target Orders: BEYQBu8Ye7b7WzNqky5TmsyomUnzsEdLCKqhYU6sCGvr (Writable)
/// #6 - Pool Coin Token Account: Raydium (SOL-RETAIL) Pool 1 (Writable)
/// #7 - Pool Pc Token Account: Raydium (SOL-RETAIL) Pool 2 (Writable)
/// #8 - Serum Program: OpenBook Program
/// #9 - Serum Market: FeEi5J2EDfKYEf9MpHi3XG55urywmC1VGfx99edn4P9t (Writable)
/// #10 - Serum Bids: 3hE83ULCHPY3gXrzYtmD7fHM7Btou7upwgSQBV68joD7 (Writable)
/// #11 - Serum Asks: D1pPxAwZCgNfUZuDT9jYLycis1aXneRbtguhNgNkHQja (Writable)
/// #12 - Serum Event Queue: 3LB32L4CFim46jbEvLJwAbRKNf4EZrxY1Mg7DP7oUG7p (Writable)
/// #13 - Serum Coin Vault Account: 6MjEX7oWjnDcPAczF2bpnPicbcWAeMt9YjUtTrQf8e5y (Writable)
/// #14 - Serum Pc Vault Account: BPKhBfBgXdputYoGW42vbijXs1czHmm3eiiD7FNdiSbM (Writable)
/// #15 - Serum Vault Signer: EHQjfmkBqziuGzL8EpZxCXTFGSXsUEomuoWMSDCYKbGk
/// #16 - User Source Token Account: 6fx8Wn8JhFiWCZMRzgXPadVKHaMtid4xD45guX4TjGGw (Writable)
/// #17 - User Destination Token Account: 38sFLvusozdxb1p2V2GovRywmF9DeLxVeVw8rK6PYe8a (Writable)
/// #18 - User Source Owner: 9AFb3BJTybJVvjWejqxstz9DUwYQxPepT94VCBi4escf (Writable, Signer, Fee Payer)
pub fn make_raydium_swap_ix(
    raydium_accounts: RaydiumAccounts,
    serum_accounts: SerumAccounts,
    user_source_token_account: Pubkey,
    user_destination_token_account: Pubkey,
    owner: Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<Instruction> {
    let accounts: [AccountMeta; 18] = [
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new(raydium_accounts.amm, false),
        AccountMeta::new_readonly(Pubkey::from_str(RAYDIUM_V4_AUTHORITY)?, false),
        AccountMeta::new(raydium_accounts.amm_open_orders, false),
        AccountMeta::new(raydium_accounts.amm_target_orders, false),
        AccountMeta::new(raydium_accounts.pool_coin_token_account, false),
        AccountMeta::new(raydium_accounts.pool_pc_token_account, false),
        AccountMeta::new_readonly(Pubkey::from_str(SERUM_PROGRAM)?, false),
        AccountMeta::new(raydium_accounts.serum_market, false),
        AccountMeta::new(serum_accounts.bids, false),
        AccountMeta::new(serum_accounts.asks, false),
        AccountMeta::new(serum_accounts.event_queue, false),
        AccountMeta::new(serum_accounts.coin_vault_account, false),
        AccountMeta::new(serum_accounts.pc_vault_account, false),
        AccountMeta::new_readonly(serum_accounts.vault_signer, false),
        AccountMeta::new(user_source_token_account, false),
        AccountMeta::new(user_destination_token_account, false),
        AccountMeta::new(owner, true),
    ];

    let data = SwapInstructionData {
        instruction: RAYDIUM_V4_BUY_METHOD,
        amount_in,
        minimum_amount_out,
    };

    Ok(Instruction::new_with_borsh(
        Pubkey::from_str(RAYDIUM_V4_PROGRAM)?,
        &data,
        accounts.to_vec(),
    ))
}
