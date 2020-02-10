use serde::{Serialize, Deserialize};
use crate::crypto::hash::{H256, Hashable};
use chrono::{DateTime, Utc};
use crate::transaction::{Transaction};
use rand;

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    header: Header,
    content: Content,
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Header{
    parent: H256,
    nonce: u32,
    difficulty: H256,
    timestamp: DateTime<Utc>,
    merkle_root: H256,
}

impl Hashable for Header{
    fn hash(&self) -> H256 {
        let bytes = bincode::serialize(&self).unwrap();
        let digest = ring::digest::digest(&ring::digest::SHA256, &bytes);
        digest.into()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Content{
    transactions: Vec<Transaction>,
}


#[cfg(any(test, test_utilities))]
pub mod test {
    use super::*;
    use crate::crypto::hash::H256;

    pub fn generate_random_block(parent: &H256) -> Block {
        Block{
            header: Header{
                parent: *parent,
                nonce: rand::random::<u32>(),
                difficulty: Default::default(),
                timestamp: Utc::now(),
                merkle_root: Default::default(),
            },
            content: Content{
                transactions: Default::default(),
            },
        }
    }
}
