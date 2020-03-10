use crate::network::server::Handle as ServerHandle;

use log::info;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;

use std::thread;

use std::sync::{Arc,Mutex};

use crate::blockchain::{Blockchain};
use crate::block::{Block, Header, Content};
use crate::crypto::merkle::{MerkleTree};
use crate::crypto::hash::{H256, Hashable};
use crate::network::message::Message;
use log::debug;

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
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(server: &ServerHandle, blockchain: &Arc<Mutex<Blockchain>>) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        mined_blocks: 0,
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
                // Get random content.
                let content = Content::new();

                // Initialize block header.
                let parent = chain.tip();
                let timestamp = time::SystemTime::now().duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_micros();
                let difficulty: H256 = chain.get_block(&parent).unwrap().header.difficulty;
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

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}
