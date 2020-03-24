use crate::network::server::Handle as ServerHandle;
use log::info;
use log::debug;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, UnparsedPublicKey, ED25519};
use std::time;
use std::thread;
use std::sync::{Arc,Mutex};
use std::collections::{LinkedList};
use crate::blockchain::{Blockchain};
use crate::block::{Block, Header, Content, State, BLOCK_CAPACITY, BLOCK_REWARD};
use crate::crypto::merkle::{MerkleTree};
use crate::crypto::hash::{H256, Hashable};
use crate::crypto::key_pair;
use crate::crypto::address::H160;
use crate::network::message::Message;
use crate::transaction::{SignedTransaction, Transaction, verify, sign};

enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
        Exit,
}

enum OperatingState {
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
    tx_mempool: Arc<Mutex<LinkedList<SignedTransaction>>>,
    id: Arc<Identity>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub struct Identity {
    /// id information about this account
    pub key_pair: Ed25519KeyPair,
    pub address: H160,
}

impl Identity {
    pub fn new() -> Identity {
        let _key_pair = key_pair::random();
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
    tx_mempool: &Arc<Mutex<LinkedList<SignedTransaction>>>,
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
        // broadcast public key
        self.server.broadcast(Message::NewAccountAddress((*self.id).address.clone()));
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
                println!("Wait for extra 3 seconds");
                thread::sleep(time::Duration::from_secs(3));
                if let Ok(chain) = self.blockchain.lock() {
                    let longest_chain = chain.all_blocks_in_longest_chain();
                    let block: Block = Default::default();
                    let bytes = bincode::serialize(&block).unwrap();
                    println!("Serialized block size: {}", bytes.len());
                    println!("Longest chain: {:?}", longest_chain);
                }
                return;
            }

            // TODO: actual mining 
            if let Ok(mut chain) = self.blockchain.lock(){
                // Initialize block header.
                let parent = chain.tip();
                let timestamp = time::SystemTime::now().duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_micros();
                let difficulty: H256 = chain.get_block(&parent).unwrap().header.difficulty;

                // Collect transactions to generate content
                if let Some(state) = chain.get_state(&parent) {
                    let (content, new_state) = self.collect_txs(&state);
                    if content.len() < BLOCK_CAPACITY {
                        continue;
                    }
                    let merkle_root = MerkleTree::new(&content.transactions).root();

                    // Create block with random nonce.
                    let block = Block {
                        header: Header{
                            parent: parent,
                            nonce: rand::random::<u32>(),
                            difficulty: difficulty,
                            timestamp: timestamp,
                            merkle_root: merkle_root,
                        },
                        content: content, 
                    };

                    // If block hash <= difficulty, block is successfully mined.
                    if block.hash() <= difficulty { 
                        self.mined_blocks += 1;
                        debug!("new block mined, hash: {:?}, num mined: {:?}", block.hash(), self.mined_blocks);
                        chain.insert(&block);
                        self.server.broadcast(Message::NewBlockHashes(vec![block.hash()]));
                    }
                }
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }

    fn collect_txs(&self, _state: &State) -> (Content, State) {
        let mut valid_transactions = vec![];
        let mut state = _state.clone();
        let mut split_index = 0;
        if let Ok(_tx_mempool) = self.tx_mempool.lock() {
            for tx_signed in _tx_mempool.iter() {
                let address: H160 = tx_signed.public_key.clone().into();
                let public_key = UnparsedPublicKey::new(&ED25519, tx_signed.public_key.clone());
                let tx = tx_signed.transaction.clone();
                split_index += 1;
                // verification fails
                if public_key.verify(tx.hash().as_ref(), tx_signed.signature.as_ref()).is_err() {
                    continue;
                }
                // get the peer state
                if let Some(peer_state) = state.account_state.get(&address) {
                    // the nonce is incorrect
                    if peer_state.nonce != (tx.account_nonce + 1) {
                        continue;
                    }
                    // the balance is not enough
                    if peer_state.balance < tx.value {
                        continue;
                    }
                    // the valid transaction
                    let mut new_state = peer_state.clone();
                    new_state.nonce = peer_state.nonce + 1;
                    new_state.balance = peer_state.balance - tx.value;
                    state.account_state.insert(address, new_state);
                    valid_transactions.push(tx_signed.clone());
                }
                if valid_transactions.len() == BLOCK_CAPACITY {
                    break;
                }
            }
        }
        if let Ok(mut _tx_mempool) = self.tx_mempool.lock() {
            
        }
        let content = Content {
            transactions: valid_transactions,
        };
        (content, state)
    }
}
