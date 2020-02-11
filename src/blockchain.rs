use crate::block::{Block, Header, Content};
use crate::crypto::hash::{H256, Hashable};
use std::collections::HashMap;

pub struct Blockchain {
    blocks: HashMap<H256,Block>,
    block_len: HashMap<H256,u32>,
    head: H256,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let genesis_block = Block {
            header: Header{
                parent: Default::default(),
                nonce: Default::default(),
                difficulty: Default::default(),
                timestamp: Default::default(),
                merkle_root: Default::default(),
            },
            content: Content{
                transactions: Default::default(),
            },
        };

        let head = genesis_block.hash();

        let mut _blocks: HashMap<H256,Block> = HashMap::new();
        _blocks.insert(head,genesis_block);

        let mut _block_len: HashMap<H256,u32> = HashMap::new();
        _block_len.insert(head,0);

        Blockchain{
            blocks: _blocks,
            block_len: _block_len,
            head: head,
        }
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let cloned_block = block.clone();
        let curr_block_hash = block.hash();
        let prev_block_hash = block.header.parent;


        if self.blocks.contains_key(&prev_block_hash){
            self.blocks.insert(curr_block_hash, cloned_block);

            let new_len: u32 = self.block_len.get(&prev_block_hash).unwrap() + 1; 
            self.block_len.insert(curr_block_hash, new_len);

            if new_len > *self.block_len.get(&self.head).unwrap(){
                self.head = curr_block_hash; 
            }
        }
        
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.head
    }

    /// Get the last block's hash of the longest chain
    #[cfg(any(test, test_utilities))]
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
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());

    }
}
