use crate::solana::{
    raydium::{get_raydium_accounts, get_serum_accounts, get_serum_market},
    util::generate_random_seed,
};
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, program_pack::Pack, pubkey::Pubkey, signer::Signer,
    system_instruction, system_program,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{self, instruction as token_instruction};

use std::str::FromStr;

use super::raydium::{
    calculate_minimum_amount_out, extract_raydium_accounts, get_raydium_pool, make_raydium_swap_ix,
};

fn apply_slippage(amount: u64, slippage_bps: u16) -> u64 {
    let slippage = amount * slippage_bps as u64 / 10_000;
    amount - slippage
}

pub async fn create_raydium_sol_swap_ix(
    pool_address: String,
    amount_in: u64,
    slippage_bps: u16,
    destination_token: Pubkey,
    rpc_client: &RpcClient,
    owner: &Pubkey,
) -> Result<Vec<Instruction>> {
    let mut ixs = vec![];

    let pool_pubkey = Pubkey::from_str(&pool_address)?;
    let pool_accounts = get_raydium_pool(rpc_client, &pool_pubkey).await?;
    // tracing::info!("RaydiumPoolLayout {:?}", pool_accounts);
    let raydium_accounts = extract_raydium_accounts(pool_pubkey, &pool_accounts);
    // tracing::info!("RaydiumAccounts {:?}", accounts);
    // let serum_market = get_serum_market(rpc_client, accounts.serum_market).await?;
    // tracing::info!("SerumMarket {:?}", serum_market);
    let serum_accounts = get_serum_accounts(rpc_client, raydium_accounts.serum_market).await?;
    // tracing::info!("SerumAccounts {:?}", serum_accounts);

    // Generate seed for temporary WSOL account
    let seed = &generate_random_seed();

    // Derive temporary WSOL account with seed
    let user_source_token_account = Pubkey::create_with_seed(owner, seed, &spl_token::id())?;

    // Calculate rent-exempt balance for token account
    let rent = rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
        .await?;

    let amount_in_with_rent = amount_in + rent;

    let minimum_amount_out =
        calculate_minimum_amount_out(&pool_accounts, amount_in_with_rent, slippage_bps as f64);

    // Create temporary WSOL account
    ixs.push(system_instruction::create_account_with_seed(
        owner,
        &user_source_token_account,
        owner,
        seed,
        amount_in + rent, // Total amount: swap amount + rent
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    ));

    // Initialize WSOL account
    ixs.push(token_instruction::initialize_account(
        &spl_token::id(),
        &user_source_token_account,
        &spl_token::native_mint::id(),
        owner,
    )?);

    // Generate user ATA for destination token
    let user_destination_token_account = get_associated_token_address(owner, &destination_token);

    ixs.push(
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &owner,
            &owner,
            &destination_token,
            &spl_token::id(),
        ),
    );

    ixs.push(make_raydium_swap_ix(
        raydium_accounts,
        serum_accounts,
        user_source_token_account,
        user_destination_token_account,
        *owner,
        amount_in,
        minimum_amount_out,
    )?);

    // 4. Close temporary WSOL account to recover rent
    ixs.push(token_instruction::close_account(
        &spl_token::id(),
        &user_source_token_account,
        owner,
        owner,
        &[owner],
    )?);

    Ok(ixs)
}
