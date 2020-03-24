use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::time;

use crate::transaction::{SignedTransaction, Transaction, sign};
use crate::network::server::Handle as ServerHandle;
use crate::network::message::Message;
use crate::crypto::hash::{Hashable, H256};
use crate::miner::Identity;
use crate::blockchain::Blockchain;
use crate::block::State;

static GEN_INTERVAL: u64 = 100;
static SEND_SIZE: usize = 2;

pub struct Context {
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    tx_mempool: Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    id: Arc<Identity>,
}

pub fn new (
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    tx_mempool: &Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    id: &Arc<Identity>,
    ) -> Context {
    let ctx = Context {
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        tx_mempool: Arc::clone(tx_mempool),
        id: Arc::clone(id),
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
        let public_key = (*self.id).key_pair.public_key();
        let self_address = (*self.id).address;
        let mut last_nonce = -1;
        loop {
            if txs_hash_buffer.len() == SEND_SIZE {
                self.server.broadcast(Message::NewTransactionHashes(txs_hash_buffer.clone()));
                txs_hash_buffer.clear();
            }
            if let Ok(mut chain) = self.blockchain.lock(){
                let tip_hash = chain.tip().hash();
                if let Some(state) = chain.get_state(&tip_hash) {
                    // get the latest state of my account
                    if let Some(self_state) = state.account_state.get(&self_address) {
                        let balance = self_state.balance;
                        let mut nonce = self_state.nonce;
                        let num_peer = state.address_list.len() - 1;
                        // already generate transactions for this block, skip
                        if last_nonce == nonce {
                            let interval = time::Duration::from_micros(GEN_INTERVAL);
                            thread::sleep(interval);
                            continue;
                        }
                        last_nonce = nonce;
                        // generate transactions for this block
                        // simply send 1/(2*num_peer) * balance to all other peers
                        for peer_address in state.address_list.iter() {
                            // skip myself
                            if peer_address == &self_address {
                                continue;
                            }
                            let tx = Transaction {
                                recipient_address: peer_address.clone(),
                                value: balance as u64 / (2 * num_peer as u64),
                                account_nonce: nonce
                            };
                            nonce += 1;
                            let signature = sign(&tx, &(*self.id).key_pair);
                            let signed_tx = SignedTransaction {
                                transaction: tx,
                                signature: signature.as_ref().iter().cloned().collect(),
                                public_key: public_key.as_ref().iter().cloned().collect()
                            };
                            txs_hash_buffer.push(signed_tx.hash());
                            if let Ok(mut mempool) = self.tx_mempool.lock() {
                                mempool.insert(signed_tx.hash(),signed_tx);
                            }
                        }
                    }
                }
            }
            let interval = time::Duration::from_micros(GEN_INTERVAL);
            thread::sleep(interval);
        }
    }
}
