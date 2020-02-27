use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};

use std::thread;
use std::sync::{Mutex, Arc};
use crate::{Blockchain, block};
use crate::crypto::hash::{Hashable, H256};
use std::collections::{HashSet,HashMap};

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    orphan_blocks: Arc<Mutex<HashMap<H256,block::Block>>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    orphan_blocks: &Arc<Mutex<HashMap<H256,block::Block>>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: blockchain.clone(),
        orphan_blocks: orphan_blocks.clone(),
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

                // If a peer advertises that it has a block that we don't have committed, request it from the peer.
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

                // If a peer asks us for a block we have committed in our blockchain, give it to them.
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

                // If we receive a block, check if it can be committed to the blockchain
                // If it can, commit it and all of its children in the orphan block pool.
                // If it can't add it to the orphan block pool and request its parent from the peer.  
                Message::Blocks(blocks) => {
                    debug!("Blocks: {:?}", blocks);
                    let mut broadcast_hashes: Vec<H256> = Vec::new();
                    let mut committed_hashes = HashSet::new();

                    if let Ok(mut orphans) = self.orphan_blocks.lock(){
                        if let Ok(mut chain) = self.blockchain.lock(){
                            for b in &blocks {
                                orphans.insert(b.hash(),b.clone());
                            }
                            loop{
                                let mut no_commits = true;
                                committed_hashes.clear();

                                for (hash,block) in &(*orphans) {
                                    if chain.insert(block) {
                                        no_commits = false;
                                        broadcast_hashes.push(*hash);
                                        committed_hashes.insert(*hash);
                                    }
                                }

                                for hash in &committed_hashes {
                                    orphans.remove(hash);

                                }

                                if no_commits {
                                    break;
                                }
                            }
                        }
                    }
                    if !broadcast_hashes.is_empty() {
                        self.server.broadcast(Message::NewBlockHashes(broadcast_hashes));
                    }
                }
            }
        }
    }
}
