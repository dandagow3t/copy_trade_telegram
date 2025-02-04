use anyhow::Result;
use async_trait::async_trait;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use std::sync::Arc;

use crate::solana::blockhash::BLOCKHASH_CACHE;
use crate::solana::transaction::{add_jito_tip, send_tx};

use super::TransactionSigner;

pub struct LocalSolanaSigner {
    keypair: Arc<Keypair>,
}

impl LocalSolanaSigner {
    pub fn new(private_key: String) -> Self {
        let keypair = Keypair::from_base58_string(&private_key);
        Self {
            keypair: Arc::new(keypair),
        }
    }
}

#[async_trait]
impl TransactionSigner for LocalSolanaSigner {
    #[cfg(feature = "evm")]
    fn address(&self) -> String {
        unimplemented!()
    }

    #[cfg(feature = "solana")]
    fn pubkey(&self) -> String {
        self.keypair.pubkey().to_string()
    }

    async fn sign_and_send_solana_transaction(
        &self,
        tx: &mut solana_sdk::transaction::Transaction,
    ) -> Result<String> {
        let recent_blockhash = BLOCKHASH_CACHE.get_blockhash().await?;
        tracing::info!("recent_blockhash: {:?}", recent_blockhash);
        tx.try_sign(&[&*self.keypair], recent_blockhash)?;
        send_tx(tx).await
    }

    async fn priority_sign_and_send_solana_transaction(
        &self,
        ix: &mut Vec<solana_sdk::instruction::Instruction>,
    ) -> Result<String> {
        let recent_blockhash = BLOCKHASH_CACHE.get_blockhash().await?;
        tracing::info!("recent_blockhash: {:?}", recent_blockhash);

        // add_priority_fee(ix, None, None).await?;
        add_jito_tip(ix, &self.keypair.pubkey());

        tracing::info!("ix: {:?}", ix);
        let mut tx = Transaction::new_with_payer(ix, Some(&self.keypair.pubkey()));
        tx.try_sign(&[&*self.keypair], recent_blockhash)?;
        send_tx(&tx).await
    }
}
