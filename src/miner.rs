use crate::network::server::Handle as ServerHandle;
use log::{info};
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use std::time;
use std::thread;
use std::sync::{Arc,Mutex};
use std::collections::{HashMap};
use crate::blockchain::{Blockchain};
use crate::block::{Block, Header, Content, State, BLOCK_CAPACITY};
use crate::crypto::merkle::{MerkleTree};
use crate::crypto::hash::{H256, Hashable};
use crate::crypto::key_pair;
use crate::crypto::address::H160;
use crate::network::message::Message;
use crate::transaction::{SignedTransaction};

pub enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
        Exit,
}

pub enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    mined_blocks: u64,
    tx_mempool: Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    id: Arc<Identity>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    pub control_chan: Sender<ControlSignal>,
}

pub struct Identity {
    /// id information about this account
    pub key_pair: Ed25519KeyPair,
    pub address: H160,
}

impl Identity {
    pub fn new(randbyte: u8) -> Identity {
        let _key_pair = key_pair::frombyte(randbyte);
        let _address: H160 = ring::digest::digest(&ring::digest::SHA256, _key_pair.public_key().as_ref()).into();
        Identity {
            key_pair: _key_pair,
            address: _address,
        }
    }
}

pub fn new(
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
        mined_blocks: 0,
        tx_mempool: Arc::clone(tx_mempool),
        id: Arc::clone(id),
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
        .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Miner shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                info!("Miner starting in continuous mode with lambda {}", i);
                self.operating_state = OperatingState::Run(i);
            }
        }
    }

    fn miner_loop(&mut self) {
        // main mining loop
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
            if let OperatingState::ShutDown = self.operating_state {
                thread::sleep(time::Duration::from_secs(3));
                if let Ok(chain) = self.blockchain.lock() {
                    let longest_chain = chain.all_blocks_in_longest_chain();
                    info!("Exit, Longest chain: {:?}", longest_chain);
                }
                return;
            }
            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }

            // TODO: actual mining 
            if let Ok(mut chain) = self.blockchain.lock(){
                // Initialize block header.
                let parent = chain.tip().clone();
                let timestamp = time::SystemTime::now().duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_micros();
                let difficulty: H256 = chain.get_block(&parent).unwrap().header.difficulty;

                // Collect transactions to generate content
                if let Some(state) = chain.get_state(&parent) {
                    let (content, new_state) = self.collect_txs(&state);
                    if content.len() == 0 {
                        continue;
                    }
                    if content.len() < BLOCK_CAPACITY {
                        continue;
                    }
                    //debug!("\r miner collected txs: {:?}", content.len());
                    let merkle_root = MerkleTree::new(&content.transactions).root();
                    // Create block with random nonce.
                    let mut block = Block {
                        header: Header{
                            parent: parent,
                            nonce: rand::random::<u32>(),
                            difficulty: difficulty,
                            timestamp: timestamp,
                            merkle_root: merkle_root,
                        },
                        content: content.clone(), 
                    };

                    for _ in 0..1000{
                        block.header.nonce = rand::random::<u32>();
                        if block.hash() < difficulty {
                            break;
                        }
                    }

                    // If block hash <= difficulty, block is successfully mined.
                    if block.hash() < difficulty {
                        info!("Mined a new block: hash: {:#?}, num transactions: {:#?}, num blocks mined: {:#?}", 
                            block.hash(), 
                            content.len(),
                            self.mined_blocks);
                        self.mined_blocks += 1;
                        chain.insert(&block, &new_state);

                        if let Ok(mut _tx_mempool) = self.tx_mempool.lock() {
                            for tx in content.transactions {
                                _tx_mempool.remove(&tx.hash());
                            }
                        }

                        self.server.broadcast(Message::NewBlockHashes(vec![block.hash()]));
                    }
                }
            }
        }
    }

    fn collect_txs(&self, _state: &State) -> (Content, State) {
        let mut valid_transactions = vec![];
        let mut erase_transactions = vec![];
        let mut state = _state.clone();

        if let Ok(mut _tx_mempool) = self.tx_mempool.lock() {
            loop{
                let mut finished = true;
                erase_transactions.clear();

                for tx_signed in _tx_mempool.values() {
                    let address: H160 = ring::digest::digest(&ring::digest::SHA256, tx_signed.public_key.as_ref()).into();
                    let public_key = UnparsedPublicKey::new(&ED25519, tx_signed.public_key.clone());
                    let tx = tx_signed.transaction.clone();
                    // verification fails
                    if public_key.verify(tx.hash().as_ref(), tx_signed.signature.as_ref()).is_err() {
                        erase_transactions.push(tx.hash());
                        continue;
                    }
                    // get the peer state
                    if let Some(peer_state) = state.account_state.get(&address) {
                        // the nonce is incorrect
                        if tx.account_nonce != peer_state.nonce+1 {
                            // only erase txs whose nonce are smaller than the state
                            if tx.account_nonce <= peer_state.nonce {
                                erase_transactions.push(tx.hash());
                            }
                            continue;
                        }
                        // the balance is not enough
                        if peer_state.balance < tx.value {
                            erase_transactions.push(tx.hash());
                            continue;
                        }
                        // the valid transaction
                        tx_signed.update_state(&mut state);
                        valid_transactions.push(tx_signed.clone());
                        finished = false;
                    }
                    if valid_transactions.len() == BLOCK_CAPACITY {
                        finished = true;
                        break;
                    }

                }

                // remove invalid txs
                for tx in erase_transactions.iter() {
                    _tx_mempool.remove(&tx.hash());
                }

                // if no more transactions can be added, return
                if finished {
                    break;
                }
            }
        }
        
        let content = Content {
            transactions: valid_transactions,
        };
        (content, state)
    }
}
