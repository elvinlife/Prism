use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};

use std::thread;
use std::sync::{Mutex, Arc};
use crate::{Blockchain, block};
use crate::crypto::hash::{Hashable, H256};
use std::collections::{HashMap};

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

                // If a peer advertises that it has a block that we don't have, request it from the peer.
                Message::NewBlockHashes(hashes) => {
                    //debug!("NewBlockHashes: {:?}", hashes);
                    let mut requested_hashes = Vec::new();

                    if let Ok(orphans) = self.orphan_blocks.lock(){
                        if let Ok(chain) = self.blockchain.lock(){ 
                            for hash in &hashes {
                                if chain.get_block(hash).is_none() && !orphans.contains_key(hash) {
                                    requested_hashes.push(*hash);
                                }
                            }
                        }
                    }

                    if !requested_hashes.is_empty() {
                        peer.write(Message::GetBlocks(requested_hashes));    
                    }
                }

                // If a peer asks us for a block we have, give it to them.
                Message::GetBlocks(hashes) => {
                    //debug!("GetBlocks: {:?}", hashes);
                    let mut blocks = Vec::new();

                    if let Ok(orphans) = self.orphan_blocks.lock(){
                        if let Ok(chain) = self.blockchain.lock() {
                            for hash in &hashes {
                                if let Some(block) = chain.get_block(hash) {
                                    blocks.push(block.clone());
                                }
                                else if let Some(block) = orphans.get(hash){
                                    blocks.push(block.clone());
                                }
                            }
                        }
                    }

                    if !blocks.is_empty() {
                        peer.write(Message::Blocks(blocks));
                    }
                }

                // If we receive a block, check if we already have it. If so dump it.
                // Otherwise the block is new. Check if we can commit it.
                // If it can, commit it and all of its children in the orphan block pool.
                // If it can't add it to the orphan block pool and request its parent from the peer if necessary.
                Message::Blocks(blocks) => {
                    //debug!("Blocks: {:?}", blocks);
                    let mut broadcast_hashes: Vec<H256> = Vec::new();
                    let mut requested_hashes: Vec<H256> = Vec::new();

                    if let Ok(mut orphans) = self.orphan_blocks.lock(){
                        if let Ok(mut chain) = self.blockchain.lock(){

                            for block in &blocks {
                                let parent_hash = block.header.parent;
                                let block_hash = block.hash();

                                // Check if already have block. If so, skip.
                                if chain.contains_key(&block_hash) || orphans.contains_key(&block_hash){
                                    continue;
                                }

                                // Otherwise block is new. Find out where the parent is.
                                if chain.contains_key(&parent_hash){
                                    // Parent in blockchain. Commit as many blocks to the chain as possible.
                                    orphans.insert(block_hash,block.clone());

                                    let mut committed_hashes = Vec::new();
                                    loop{
                                        // Reset everything
                                        let mut no_commits = true;
                                        committed_hashes.clear();

                                        // Loop through orphan pool and commit as many blocks as possible.
                                        for (block_hash, block) in orphans.iter() {
                                            let parent_hash = block.header.parent;

                                            // Commit if parent in blockchain and nonce is valid.
                                            if chain.contains_key(&parent_hash)
                                               && block_hash <= &chain.get_block(&parent_hash).unwrap().header.difficulty {
                                                chain.insert(&block);
                                                no_commits = false;
                                                committed_hashes.push(*block_hash);
                                                broadcast_hashes.push(*block_hash);
                                            }
                                        }

                                        // Clear all committed blocks from orphan pool.
                                        for hash in &committed_hashes {
                                            orphans.remove(&hash);
                                        }

                                        // Repeat until convergence.
                                        if no_commits {
                                            break;
                                        }
                                    }                                   
                                }
                                else if orphans.contains_key(&parent_hash){
                                    // Parent is also orphan, So block is orphan, don't request parent.
                                    orphans.insert(block_hash,block.clone());
                                }
                                else{
                                    // Parent doesn't exist. So block is orphan, request parent.
                                    orphans.insert(block_hash,block.clone());
                                    requested_hashes.push(parent_hash);
                                }
                            }
                        }
                    }
                    // Announce committed hashes.
                    if !broadcast_hashes.is_empty() {
                        self.server.broadcast(Message::NewBlockHashes(broadcast_hashes));
                    }
                    // Get orphan block parents from peer.
                    if !requested_hashes.is_empty() {
                        peer.write(Message::GetBlocks(requested_hashes));
                    }
                }
            }
        }
    }
}
