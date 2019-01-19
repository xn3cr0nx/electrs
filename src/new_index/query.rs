use bitcoin::util::hash::Sha256dHash;
use rayon::prelude::*;

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc,RwLock};

use crate::chain::{OutPoint, Transaction, TxOut};
use crate::new_index::{ChainQuery, Mempool, ScriptStats, SpendingInput, Utxo};
use crate::util::{Bytes, TransactionStatus};

pub struct Query {
    pub chain: Arc<ChainQuery>, // TODO: should be used as read-only
    pub mempool: Arc<RwLock<Mempool>>,
}

impl Query {
    pub fn new(chain: Arc<ChainQuery>, mempool: Arc<RwLock<Mempool>>) -> Self {
        Query { chain, mempool }
    }

    pub fn utxo(&self, scripthash: &[u8]) -> Vec<Utxo> {
        let mut utxos = self.chain.utxo(scripthash);
        utxos.extend(self.mempool.read().unwrap().utxo(scripthash));
        utxos
    }

    pub fn stats(&self, scripthash: &[u8]) -> (ScriptStats, ScriptStats) {
        (self.chain.stats(scripthash), self.mempool.read().unwrap().stats(scripthash))
    }

    pub fn lookup_txn(&self, txid: &Sha256dHash) -> Option<Transaction> {
        self.chain
            .lookup_txn(txid)
            .or_else(|| self.mempool.read().unwrap().lookup_txn(txid))
    }
    pub fn lookup_raw_txn(&self, txid: &Sha256dHash) -> Option<Bytes> {
        self.chain
            .lookup_raw_txn(txid)
            .or_else(|| self.mempool.read().unwrap().lookup_raw_txn(txid))
    }

    pub fn lookup_txos(&self, outpoints: &BTreeSet<OutPoint>) -> HashMap<OutPoint, TxOut> {
        // the mempool lookup_txos() internally looks up confirmed txos as well
        self.mempool.read().unwrap()
            .lookup_txos(outpoints)
            .expect("failed loading txos")
    }

    pub fn lookup_spend(&self, outpoint: &OutPoint) -> Option<SpendingInput> {
        self.chain
            .lookup_spend(outpoint)
            .or_else(|| self.mempool.read().unwrap().lookup_spend(outpoint))
    }

    pub fn lookup_tx_spends(&self, tx: Transaction) -> Vec<Option<SpendingInput>> {
        let txid = tx.txid();

        tx.output
            .par_iter()
            .enumerate()
            .map(|(vout, txout)| {
                if !txout.script_pubkey.is_provably_unspendable() {
                    self.lookup_spend(&OutPoint {
                        txid,
                        vout: vout as u32,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_tx_status(&self, txid: &Sha256dHash) -> TransactionStatus {
        TransactionStatus::from(self.chain.tx_confirming_block(txid))
    }
}