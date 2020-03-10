use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::LinkedList;
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::time;

use crate::transaction::{SignedTransaction, Transaction, sign};
use crate::network::server::Handle as ServerHandle;
use crate::network::message::Message;
use crate::key_pair;
use crate::crypto::hash::{Hashable, H256};

static GEN_INTERVAL: u64 = 100;
static SEND_SIZE: usize = 10;

pub struct Context {
    server: ServerHandle,
    tx_mempool: Arc<Mutex<LinkedList<SignedTransaction>>>,
    key_pair: Ed25519KeyPair,
}

pub fn new (
    server: &ServerHandle,
    tx_mempool: &Arc<Mutex<LinkedList<SignedTransaction>>>,
) -> Context {
    let key_pair = key_pair::random();
    let ctx = Context {
        server: server.clone(),
        tx_mempool: Arc::clone(tx_mempool),
        key_pair: key_pair,
    };
    ctx
}

impl Context {
    pub fn start(self) {
        thread::spawn(move || {
            self.gen_loop();
        });
    }

    pub fn gen_loop(self) {
        let mut txs_hash_buffer: Vec<H256> = Vec::new();
        let public_key = self.key_pair.public_key();
        loop {
            if txs_hash_buffer.len() == SEND_SIZE {
                self.server.broadcast(Message::NewTransactionHashes(txs_hash_buffer.clone()));
                txs_hash_buffer.clear();
            }
            let tx: Transaction = Default::default();
            let signature = sign(&tx, &self.key_pair);
            let signed_tx = SignedTransaction {
                transaction: tx,
                signature: signature.as_ref().iter().cloned().collect(),
                public_key: public_key.as_ref().iter().cloned().collect()
            };
            txs_hash_buffer.push(signed_tx.hash());
            if let Ok(mut mempool) = self.tx_mempool.lock() {
                mempool.push_back(signed_tx);
            }
            let interval = time::Duration::from_micros(GEN_INTERVAL);
            thread::sleep(interval);
        }
    }
}