use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};

use std::thread;
use std::sync::{Mutex, Arc};
use std::collections::{HashMap};
use std::time;
use crate::{Blockchain, block::{Block, State, AccountState}};
use crate::crypto::hash::{Hashable, H256};
use crate::crypto::address::H160;
use crate::transaction::{SignedTransaction,verify};
use ring::signature::{UnparsedPublicKey, ED25519};
//use std::sync::atomic::{AtomicU128, Ordering, AtomicU32};


#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    orphan_blocks: Arc<Mutex<HashMap<H256,Block>>>,
    tx_mempool: Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    delay_time_sum: Arc<Mutex<u128>>,
    recv_block_sum: Arc<Mutex<u32>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    orphan_blocks: &Arc<Mutex<HashMap<H256,Block>>>,
    tx_mempool: &Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    delay_time_sum: &Arc<Mutex<u128>>,
    recv_block_sum: &Arc<Mutex<u32>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: blockchain.clone(),
        orphan_blocks: orphan_blocks.clone(),
        tx_mempool: tx_mempool.clone(),
        delay_time_sum: Arc::clone(delay_time_sum),
        recv_block_sum: Arc::clone(recv_block_sum),
    }
}

 // verify a block wrt the state
    // If the block is valid, return the updated state
    fn verify_block(block: &Block, _state: &State) -> Option<State> {
        let mut txs_map = HashMap::<H160, Vec<SignedTransaction>>::new();
        let address_list = _state.clone().address_list;
        let mut state = _state.clone();
        for address in address_list.iter() {
            let txs = vec![];
            txs_map.insert(address.clone(), txs);
        }
        for tx in block.content.transactions.iter() {
            let address: H160 = ring::digest::digest(&ring::digest::SHA256, tx.public_key.as_ref()).into();
            if let Some(mut _txs) = txs_map.get_mut(&address) {
                _txs.push(tx.clone());
            }
        }
        // sort it by the nonce
        for address in address_list.iter() {
            if let Some(mut _txs) = txs_map.get_mut(address) {
                _txs.sort_by(|a, b| a.transaction.account_nonce.cmp(&b.transaction.account_nonce));
                for tx in _txs.iter() {
                    if !tx.is_valid(&state) {
                        return None;
                    }
                    tx.update_state(&mut state);
                }
            }
        }
        return Some(state);
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
                    //let mut requested_hashes = Vec::new();

                    for hash in &hashes {
                        if let Ok(chain) = self.blockchain.lock(){ 
                            if let Ok(orphans) = self.orphan_blocks.lock(){
                                if chain.get_block(hash).is_none() && !orphans.contains_key(hash) {
                                    //requested_hashes.push(*hash);
                                    //peer.write(Message::GetBlocks(vec![*hash]));
                                    self.server.broadcast(Message::GetBlocks(vec![*hash]));
                                }
                            }
                        }
                    }

                    /*
                    if !requested_hashes.is_empty() {
                        peer.write(Message::GetBlocks(requested_hashes));    
                    }
                    */
                }

                // If a peer asks us for a block we have, give it to them.
                Message::GetBlocks(hashes) => {
                    //debug!("GetBlocks: {:?}", hashes);
                    //let mut blocks = Vec::new();

                    for hash in &hashes {
                        if let Ok(chain) = self.blockchain.lock() {
                            if let Ok(orphans) = self.orphan_blocks.lock(){
                                if let Some(block) = chain.get_block(hash) {
                                    //blocks.push(block.clone());
                                    peer.write(Message::Blocks(vec![block.clone()]));
                                }
                                else if let Some(block) = orphans.get(hash){
                                    //blocks.push(block.clone());
                                    peer.write(Message::Blocks(vec![block.clone()]));
                                }
                            }
                        }
                    }

                    /*
                    if !blocks.is_empty() {
                        peer.write(Message::Blocks(blocks));
                    }
                    */
                }

                // If we receive a block, check if we already have it. If so dump it.
                // Otherwise the block is new. Check if we can commit it.
                // If it can, commit it and all of its children in the orphan block pool.
                // If it can't add it to the orphan block pool and request its parent from the peer if necessary.
                Message::Blocks(blocks) => {
                    //debug!("Blocks: {:?}", blocks);

                    //let mut broadcast_hashes: Vec<H256> = Vec::new();
                    let timestamp_rcv = time::SystemTime::now().duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_micros();
                    
                    {
                        let mut delay = self.delay_time_sum.lock().unwrap();
                        let mut num = self.recv_block_sum.lock().unwrap();
                        for block in &blocks {
                            *delay += timestamp_rcv - block.header.timestamp;
                            *num += 1;
                            //broadcast_hashes.push(block.hash());
                            self.server.broadcast(Message::NewBlockHashes(vec![block.hash()]));
                        }
                        //println!("Block recv ave latency: {}", *delay as f64 / *num as f64);
                    }

                    // Fast relay blocks
                    /*
                    if !broadcast_hashes.is_empty() {
                        self.server.broadcast(Message::NewBlockHashes(broadcast_hashes));
                    }
                    */

                    //let mut requested_hashes: Vec<H256> = Vec::new();
                    for block in &blocks {
                        if let Ok(mut chain) = self.blockchain.lock(){
                            if let Ok(mut orphans) = self.orphan_blocks.lock(){

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
                                                let parent_state = chain.get_state(&parent_hash).unwrap();
                                                match verify_block(block, parent_state) {
                                                    Some(new_state) => {
                                                        no_commits = false;
                                                        chain.insert(&block, &new_state);

                                                        // If added block is not stale, drain its txns from the tx_mempool.
                                                        if parent_hash == *chain.tip(){
                                                            if let Ok(mut _tx_mempool) = self.tx_mempool.lock() {
                                                                for tx in block.content.transactions.iter() {
                                                                    _tx_mempool.remove(&tx.hash());
                                                                }
                                                            }
                                                        }

                                                        committed_hashes.push(*block_hash);
                                                    }
                                                    None => {
                                                    }
                                                }
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
                                    //requested_hashes.push(parent_hash);
                                    peer.write(Message::GetBlocks(vec![parent_hash]));
                                }
                            }
                        }
                    }
                    // Get orphan block parents from peer.
                    /*
                    if !requested_hashes.is_empty() {
                        peer.write(Message::GetBlocks(requested_hashes));
                    }
                    */
                }

                // If a peer advertises that it has a transaction that we don't have, request it from the peer.
                Message::NewTransactionHashes(hashes) => {
                    //debug!("message: NewTransactionHashes: {:?}", hashes);
                    //let mut requested_hashes = Vec::new();

                    for hash in &hashes {
                        if let Ok(tx_pool) = self.tx_mempool.lock(){
                            if !tx_pool.contains_key(hash) {
                                //requested_hashes.push(*hash);
                                //peer.write(Message::GetTransactions(vec![hash.clone()]));
                                self.server.broadcast(Message::GetTransactions(vec![hash.clone()]));
                            }
                        }
                    }

                    /*
                    if !requested_hashes.is_empty() {
                        peer.write(Message::GetTransactions(requested_hashes));    
                    }
                    */
                }

                // If a peer requests a transaction that we have in our pool, give it to them.
                Message::GetTransactions(hashes) => {
                    //debug!("message: GetTransactions: {:?}", hashes);
                    //let mut txs = Vec::new();

                    for hash in &hashes {
                        if let Ok(tx_pool) = self.tx_mempool.lock(){
                            if let Some(tx) = tx_pool.get(hash){
                                //txs.push(tx.clone());
                                peer.write(Message::Transactions(vec![tx.clone()]));
                            }
                        }
                    }

                    /*
                    if !txs.is_empty() {
                        peer.write(Message::Transactions(txs));
                    }
                    */
                }

                // If transaction received, check if we have it. If so dump it
                // Otherwise transaction is new. Check if it is signed correctly
                // If so, add it to tx_mempool and rebroadcast it.
                Message::Transactions(signed_transactions) => {
                    //debug!("message: Transactions: {:?}", signed_transactions);

                    for tx_signed in signed_transactions {

                        // Check if it is signed correctly. If not ignore it.
                        let tx = tx_signed.transaction.clone();
                        let public_key = UnparsedPublicKey::new(&ED25519, tx_signed.public_key.clone());
                        if public_key.verify(tx.hash().as_ref(), tx_signed.signature.as_ref()).is_ok() {

                            // If this is a new transaction, insert it and rebroadcast it.
                            if let Ok(mut _tx_mempool) = self.tx_mempool.lock(){
                                if !_tx_mempool.contains_key(&tx_signed.hash()){
                                    //debug!("insert from message: sender_pub: {:?}, tx: {:?}", tx_signed.public_key, tx_signed.transaction.clone());
                                    _tx_mempool.insert(tx_signed.hash(), tx_signed.clone());
                                    self.server.broadcast(Message::Transactions(vec![tx_signed]));
                                    //debug!("tx_pool size: {:?}", _tx_mempool.len());
                                }
                            }

                        }
                    }

                    /*
                    if !broadcast_hashes.is_empty() {
                        self.server.broadcast(Message::NewTransactionHashes(broadcast_hashes));
                    }
                    */
                }
            }
        }
    }
}
