use anyhow::Result;
use async_trait::async_trait;

use crate::solana::blockhash::BLOCKHASH_CACHE;
use crate::wallet_manager::{UserSession, WalletManager};
use std::sync::Arc;

use super::TransactionSigner;

pub struct PrivySigner {
    wallet_manager: Arc<WalletManager>,
    session: UserSession,
}

impl PrivySigner {
    pub fn new(
        wallet_manager: Arc<WalletManager>,
        session: UserSession,
    ) -> Self {
        Self {
            wallet_manager,
            session,
        }
    }
}

#[async_trait]
impl TransactionSigner for PrivySigner {
    fn address(&self) -> String {
        self.session.wallet_address.clone()
    }

    fn pubkey(&self) -> String {
        self.session.pubkey.clone()
    }

    #[cfg(feature = "solana")]
    async fn sign_and_send_solana_transaction(
        &self,
        tx: &mut solana_sdk::transaction::Transaction,
    ) -> Result<String> {
        tx.message.recent_blockhash = BLOCKHASH_CACHE.get_blockhash().await?;
        let tx_hash = self
            .wallet_manager
            .sign_and_send_solana_transaction(
                self.session.wallet_address.clone(),
                tx,
            )
            .await?;
        Ok(tx_hash)
    }

    #[cfg(feature = "evm")]
    async fn sign_and_send_evm_transaction(
        &self,
        tx: alloy::rpc::types::TransactionRequest,
    ) -> Result<String> {
        let tx_hash = self
            .wallet_manager
            .sign_and_send_evm_transaction(
                self.session.wallet_address.clone(),
                tx,
            )
            .await?;
        Ok(tx_hash)
    }
}
