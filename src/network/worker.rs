use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};

use std::thread;
use std::sync::{Mutex, Arc};
use crate::{Blockchain, block};
use crate::crypto::hash::Hashable;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    orphan_blocks: Vec<block::Block>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: blockchain.clone(),
        orphan_blocks: vec![]
    }
}

impl Context {
    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let mut cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&mut self) {
        loop {
            let msg = self.msg_chan.recv().unwrap();
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(hashes) => {
                    debug!("NewBlockHashes: {:?}", hashes);
                    let mut claim_hashes = Vec::new();
                    if let Ok(chain) = self.blockchain.lock(){ 
                        for hash in &hashes {
                            if chain.get_block(hash).is_none() {
                                claim_hashes.push(hash.clone());
                            }
                        }
                    }
                    if !claim_hashes.is_empty() {
                        peer.write(Message::GetBlocks(claim_hashes));    
                    }
                }
                Message::GetBlocks(hashes) => {
                    debug!("GetBlocks: {:?}", hashes);
                    let mut blocks = Vec::new();
                    if let Ok(chain) = self.blockchain.lock() {
                        for hash in &hashes {
                            if let Some(block) = chain.get_block(hash) {
                                blocks.push(block.clone());
                            }
                        }
                    }
                    if !blocks.is_empty() {
                        peer.write(Message::Blocks(blocks));
                    }
                }
                Message::Blocks(blocks) => {
                    debug!("Blocks: {:?}", blocks);
                    let mut broadcast_hashes = Vec::new();
                    if let Ok(mut chain) = self.blockchain.lock() {
                        for b in &blocks {
                            if chain.insert(b) {
                                broadcast_hashes.push(b.hash());
                            }
                            else {
                                debug!("Orphan block inserted, hash: {:?}", b.hash());
                                self.orphan_blocks.push(b.clone());
                            }
                        }
                        self.orphan_blocks.retain(|x| chain.insert(&x));
                    }
                    if !broadcast_hashes.is_empty() {
                        self.server.broadcast(Message::NewBlockHashes(broadcast_hashes));
                    }
                }
            }
        }
    }
}
