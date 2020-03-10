use serde::{Serialize, Deserialize};
use crate::crypto::hash::{H256, Hashable};
use crate::transaction::{SignedTransaction};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Block {
    #[inline]
    fn add_tx(mut self, tx: SignedTransaction) {
        self.content.transactions.push(tx);
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct Header{
    pub parent: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128,
    pub merkle_root: H256,
}

impl Hashable for Header{
    fn hash(&self) -> H256 {
        let bytes = bincode::serialize(&self).unwrap();
        let digest = ring::digest::digest(&ring::digest::SHA256, &bytes);
        digest.into()
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Content{
    pub transactions: Vec<SignedTransaction>,
}

impl Content{
    pub fn new() -> Self {
        Content{
            transactions: Default::default(),
        }
    }
}


#[cfg(any(test, test_utilities))]
pub mod test {
    use super::*;
    use crate::crypto::hash::H256;

    pub fn generate_random_block(parent: &H256) -> Block { 
        Block {
            header: Header{
                parent: parent.clone(),
                nonce: rand::random::<u32>(),
                difficulty: Default::default(),
                timestamp: Default::default(),
                merkle_root: Default::default(),
            },
            content: Content{
                transactions: Default::default(),
            }
        }
    }
}
