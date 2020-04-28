use crate::block::{Block, Header, Content, State, INIT_COINS, AccountState};
use crate::crypto::hash::{H256, Hashable};
use crate::crypto::address::H160;
use crate::crypto::key_pair;
use ring::signature::KeyPair;
use std::collections::HashMap;
use log::info;

pub struct Blockchain {
    blocks: HashMap<H256,Block>,
    block_len: HashMap<H256,u32>,
    block_states: HashMap<H256, State>,
    head: H256,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let genesis_block = Block {
            header: Header{
                parent: Default::default(),
                nonce: Default::default(),
                difficulty: H256::from([16,0,0,0,0,0,0,0,
                                        0,0,0,0,0,0,0,0,
                                        0,0,0,0,0,0,0,0,
                                        0,0,0,0,0,0,0,0]),
                timestamp: Default::default(),
                merkle_root: Default::default(),
            },
            content: Content{
                transactions: Default::default(),
            },
        };

        let mut address_list = Vec::new();
        let mut account_state: HashMap<H160, AccountState> = HashMap::new();
        for i in 0..3 {
            let key_pair = key_pair::frombyte(i as u8);
            let address: H160 = ring::digest::digest(&ring::digest::SHA256, key_pair.public_key().as_ref()).into();
            address_list.push(address);
            account_state.insert(address, AccountState{
                balance: INIT_COINS,
                nonce: 0,
            });
        }
        info!("ICO: address0: {:?}, balance: {}; address1: {:?}, balance: {}; address2: {:?}, balance: {}", 
            address_list[0], INIT_COINS, address_list[1], INIT_COINS, address_list[2], INIT_COINS);
        let genesis_state = State {
            address_list: address_list,
            account_state: account_state,
        };

        let head = genesis_block.hash();

        let mut _blocks: HashMap<H256,Block> = HashMap::new();
        _blocks.insert(head,genesis_block);

        let mut _block_len: HashMap<H256,u32> = HashMap::new();
        _block_len.insert(head,1);

        let mut _block_state: HashMap<H256, State> = HashMap::new();
        _block_state.insert(head, genesis_state);

        Blockchain{
            blocks: _blocks,
            block_len: _block_len,
            head: head,
            block_states: _block_state,
        }
    }

    /// Insert a block & the state into blockchain
    pub fn insert(&mut self, block: &Block, state: &State) -> bool{
        let curr_block_hash = block.hash();
        let prev_block_hash = block.header.parent;

        if let Some(_) = self.blocks.get(&prev_block_hash){
            self.blocks.insert(curr_block_hash, block.clone());

            let new_len: u32 = self.block_len.get(&prev_block_hash).unwrap() + 1; 
            self.block_len.insert(curr_block_hash, new_len);

            self.block_states.insert(curr_block_hash, state.clone());

            if new_len > *self.block_len.get(&self.head).unwrap(){
                self.head = curr_block_hash; 
            }

            info!("New block_hash: {:?} \r\n total blocks: {:?}, longest_chain_len: {:?}, \r\n Blockchain: tip_hash: {:?}, tip state: {:#?}; ",
                block.hash(), self.blocks.len(), self.block_len.get(self.tip()).unwrap(), self.tip(), state.account_state);
            return true;
        }
        false
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> &H256 {
        &self.head
    }

    pub fn get_block(&self, hash: &H256) -> Option<&Block> {
        self.blocks.get(&hash)
    }

    pub fn get_state(&self, hash: &H256) -> Option<& State> {
        self.block_states.get(hash)
    }

    pub fn update_state(&mut self, hash: &H256, state: &State) {
        self.block_states.insert(hash.clone(), state.clone());
    }

    pub fn contains_key(&self, hash: &H256) -> bool{
        self.blocks.contains_key(&hash)
    }

    /// Get the last block's hash of the longest chain
    //#[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut longest_chain = Vec::<H256>::new();

        let mut curr = self.head;

        while self.blocks.contains_key(&curr){
            longest_chain.push(curr);
            curr = self.blocks.get(&curr).unwrap().header.parent;
        }

        longest_chain
    }
}

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::block::test::generate_random_block;
    use crate::crypto::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new(Default::default());
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());

    }

    #[test]
    fn test_longest_chain() {
        let mut blockchain = Blockchain::new(Default::default());
        let hash_0 = blockchain.tip();
        let mut block1 = generate_random_block(&hash_0);
        let mut block2 = generate_random_block(&hash_0);
        let mut chain_correct = Vec::<H256>::new();
        chain_correct.push(hash_0);
        for _ in 0..20 {
            blockchain.insert(&block1, Default::default());
            blockchain.insert(&block2, Default::default());
            chain_correct.push(block1.hash());
            block1 = generate_random_block(&block1.hash());
            block2 = generate_random_block(&block2.hash());
        }
        chain_correct.reverse();
        let chain_to_verify = blockchain.all_blocks_in_longest_chain();
        assert_eq!(chain_to_verify, chain_correct);
    } 
}
