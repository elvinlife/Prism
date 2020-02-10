use serde::{Serialize, Deserialize};
use crate::crypto::hash::{H256, Hashable};
use chrono::{DateTime, Utc};
use crate::transaction::{Transaction};

#[derive(Serialize, Deserialize, Debug)]
pub struct Header{
    parent: H256,
    nonce: u32,
    difficulty: H256,
    timestamp: DateTime<Utc>,
    merkle_root: H256,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Content{
    transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    header: Header,
    content: Content,
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        unimplemented!()
    }
}

#[cfg(any(test, test_utilities))]
pub mod test {
    use super::*;
    use crate::crypto::hash::H256;

    pub fn generate_random_block(parent: &H256) -> Block {
        unimplemented!()
    }
}
