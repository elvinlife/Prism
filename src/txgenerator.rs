use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::time;
use rand::Rng;
use log::{info, debug};
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use crate::transaction::{SignedTransaction, Transaction, sign};
use crate::network::server::Handle as ServerHandle;
use crate::network::message::Message;
use crate::crypto::hash::{Hashable, H256};
use crate::crypto::address::H160;
use crate::miner::{Identity, OperatingState, ControlSignal, Handle};
use crate::blockchain::{Blockchain};
use rand::seq::IteratorRandom;
use rand::thread_rng;

static GEN_INTERVAL: u64 = 5000000;
pub static TX_MEMPOOL_CAPACITY: usize = 10;

pub struct Context {
    server: ServerHandle,
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    blockchain: Arc<Mutex<Blockchain>>,
    tx_mempool: Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    id: Arc<Identity>,
}

pub fn new (
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    tx_mempool: &Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    id: &Arc<Identity>,
    ) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        tx_mempool: Arc::clone(tx_mempool),
        id: Arc::clone(id),
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("txgenerator".to_string())
            .spawn(move || {
                self.gen_loop();
            })
        .unwrap();
        info!("Txgenerator initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("TXgenerator shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                info!("TXgenerator starting in continuous mode with lambda {}", i);
                self.operating_state = OperatingState::Run(i);
            }
        }
    }

    pub fn gen_loop(&mut self) {
        let mut txs_hash_buffer: Vec<H256> = Vec::new();
        let _id = self.id.clone();
        let public_key = (*_id).key_pair.public_key();
        let self_address = (*_id).address;
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    self.handle_control_signal(signal);
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        self.handle_control_signal(signal);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            /*
            if txs_hash_buffer.len() >= SEND_SIZE {
                self.server.broadcast(Message::NewTransactionHashes(txs_hash_buffer.clone()));
                txs_hash_buffer.clear();
            }
            */
            if let Ok(chain) = self.blockchain.lock(){
                let tip_hash = chain.tip();
                if let Some(state) = chain.get_state(&tip_hash) {
                    // get the latest state of my account
                    if let Some(self_state) = state.account_state.get(&self_address) {
                        let balance = self_state.balance;
                        let nonce = self_state.nonce;
                        // already generate transactions for this block, skip
                        // if last_nonce == nonce {
                        //     let interval = time::Duration::from_micros(GEN_INTERVAL);
                        //     thread::sleep(interval);
                        //     continue;
                        // }
                        // last_nonce = nonce;
                        // generate transactions for this block
                        // simply send 1/(2*num_peer) * balance to all other peers
                        let mut peer_address: Vec<H160> = Vec::new();
                        for address in state.address_list.iter() {
                            if address == &self_address {
                                continue;
                            }
                            peer_address.push(address.clone());
                        }
                        let mut rng = rand::thread_rng();
                        let receiver = peer_address[rng.gen_range(0, peer_address.len())];
                        let tx = Transaction {
                            recipient_address: receiver,
                            value: balance as u64 / 2,
                            account_nonce: nonce+1
                        };
                        let signature = sign(&tx, &(*self.id).key_pair);
                        let signed_tx = SignedTransaction {
                            transaction: tx,
                            signature: signature.as_ref().iter().cloned().collect(),
                            public_key: public_key.as_ref().iter().cloned().collect()
                        };
                        //txs_hash_buffer.push(signed_tx.hash());

                        //info!("Generate Tx: {:#?}", signed_tx.transaction);
                        if let Ok(mut _tx_mempool) = self.tx_mempool.lock() {
                            if _tx_mempool.len() >= TX_MEMPOOL_CAPACITY{
                                let random_key = {
                                    let mut rng = thread_rng();
                                    _tx_mempool.keys().choose(&mut rng).unwrap().clone()
                                };
                                _tx_mempool.remove(&random_key);
                            }
                            _tx_mempool.insert(signed_tx.hash(), signed_tx.clone());
                            self.server.broadcast(Message::Transactions(vec![signed_tx]));
                            //debug!("tx_pool size: {:?}", _tx_mempool.len());
                            //self.server.broadcast(Message::NewTransactionHashes(vec![signed_tx.hash()]));
                        }
                    }
                }
            }
            let interval = time::Duration::from_micros(GEN_INTERVAL);
            thread::sleep(interval);
        }
    }
}
