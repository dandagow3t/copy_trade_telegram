use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use std::str::FromStr;

use super::raydium::get_raydium_pool;

fn apply_slippage(amount: u64, slippage_bps: u16) -> u64 {
    let slippage = amount * slippage_bps as u64 / 10_000;
    amount - slippage
}

pub async fn create_raydium_swap_ix(
    pool_address: String,
    amount_in: u64,
    slippage_bps: u16,
    minimum_amount_out: u64,
    rpc_client: &RpcClient,
    owner: &Pubkey,
) -> Result<Vec<Instruction>> {
    let pool_pubkey = Pubkey::from_str(&pool_address)?;
    let pool_accounts = get_raydium_pool(rpc_client, pool_pubkey).await?;
    tracing::info!("RaydiumPoolLayout {:#?}", pool_accounts);
    // let swap_ix = make_raydium_swap_ix(
    //     &pool_accounts,
    //     source_token_account,
    //     destination_token_account,
    //     *owner,
    //     amount_in,
    //     minimum_amount_out,
    // )?;

    Ok(vec![])
}

// pub async fn create_raydium_swap_tx(
//     pool_address: String,
//     amount_in: u64,
//     minimum_amount_out: u64,
//     source_token_account: Pubkey,
//     destination_token_account: Pubkey,
//     owner: &Pubkey,
// ) -> Result<Transaction> {
//     let swap_ix = create_raydium_swap_ix(
//         pool_address,
//         amount_in,
//         minimum_amount_out,
//         source_token_account,
//         destination_token_account,
//         owner,
//     )
//     .await?;

//     let tx = Transaction::new_with_payer(swap_ix.as_slice(), Some(owner));
//     Ok(tx)
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::solana::util::{make_rpc_client, make_test_signer};
//     use solana_sdk::native_token::sol_to_lamports;

//     #[tokio::test]
//     async fn test_raydium_swap() {
//         let signer = make_test_signer();
//         let owner = Pubkey::from_str(&signer.pubkey()).unwrap();

//         // Example test values - these would need to be replaced with real accounts
//         let pool_address = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string();
//         let source_token =
//             Pubkey::from_str("6fx8Wn8JhFiWCZMRzgXPadVKHaMtid4xD45guX4TjGGw").unwrap();
//         let dest_token = Pubkey::from_str("38sFLvusozdxb1p2V2GovRywmF9DeLxVeVw8rK6PYe8a").unwrap();

//         let mut tx = create_raydium_swap_tx(
//             pool_address,
//             sol_to_lamports(0.05), // 0.05 SOL
//             2823548469,            // Minimum amount out
//             source_token,
//             dest_token,
//             &owner,
//         )
//         .await
//         .unwrap();

//         let result = signer.sign_and_send_solana_transaction(&mut tx).await;
//         assert!(result.is_ok(), "{:?}", result);
//     }
// }
