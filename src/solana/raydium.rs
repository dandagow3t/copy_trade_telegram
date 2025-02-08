use borsh::{BorshDeserialize, BorshSerialize};
use log::{debug, error, warn};
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

pub async fn get_raydium_pool(
    rpc_client: &RpcClient,
    raydium_pool_pubkey: Pubkey,
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

#[derive(Debug)]
pub struct RaydiumAccounts {
    pub amm: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub pool_coin_token_account: Pubkey,
    pub pool_pc_token_account: Pubkey,
    pub serum_program: Pubkey,
    pub serum_market: Pubkey,
    pub serum_bids: Pubkey,
    pub serum_asks: Pubkey,
    pub serum_event_queue: Pubkey,
    pub serum_coin_vault_account: Pubkey,
    pub serum_pc_vault_account: Pubkey,
    pub serum_vault_signer: Pubkey,
}

#[derive(Debug)]
pub struct SwapInstructionData {
    pub instruction: u8,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
}

impl SwapInstructionData {
    pub fn new(amount_in: u64, minimum_amount_out: u64) -> Self {
        Self {
            instruction: 9, // Swap instruction discriminator
            amount_in,
            minimum_amount_out,
        }
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(17);
        data.push(self.instruction);
        data.extend_from_slice(&self.amount_in.to_le_bytes());
        data.extend_from_slice(&self.minimum_amount_out.to_le_bytes());
        data
    }
}

pub const RAYDIUM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

pub fn make_raydium_swap_ix(
    accounts: &RaydiumAccounts,
    user_source_token_account: Pubkey,
    user_destination_token_account: Pubkey,
    user_authority: Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(RAYDIUM_V4_PROGRAM_ID)?;
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ID)?;

    let accounts = vec![
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new(accounts.amm, false),
        AccountMeta::new_readonly(accounts.amm_authority, false),
        AccountMeta::new(accounts.amm_open_orders, false),
        AccountMeta::new(accounts.amm_target_orders, false),
        AccountMeta::new(accounts.pool_coin_token_account, false),
        AccountMeta::new(accounts.pool_pc_token_account, false),
        AccountMeta::new_readonly(accounts.serum_program, false),
        AccountMeta::new(accounts.serum_market, false),
        AccountMeta::new(accounts.serum_bids, false),
        AccountMeta::new(accounts.serum_asks, false),
        AccountMeta::new(accounts.serum_event_queue, false),
        AccountMeta::new(accounts.serum_coin_vault_account, false),
        AccountMeta::new(accounts.serum_pc_vault_account, false),
        AccountMeta::new_readonly(accounts.serum_vault_signer, false),
        AccountMeta::new(user_source_token_account, false),
        AccountMeta::new(user_destination_token_account, false),
        AccountMeta::new(user_authority, true),
    ];

    let data = SwapInstructionData::new(amount_in, minimum_amount_out).pack();

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}
