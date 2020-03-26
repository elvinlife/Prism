use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, UnparsedPublicKey, ED25519};
use crate::crypto::hash::{H256, Hashable};
use crate::crypto::address::{H160};
use crate::block::State;

// Account based model transaction (Ethereum).
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub recipient_address: H160,
    pub value: u64,
    pub account_nonce: i32,
}

// UTXO based transaction
/*
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
prev_tx_hash: H256,
index: i8,
recipient_address: H160,
value:  u32,
}
*/

impl Hashable for Transaction{
    fn hash(&self) -> H256 {
        let t_bytes = bincode::serialize(&self).unwrap();
        let t_digest = ring::digest::digest(&ring::digest::SHA256, &t_bytes);
        t_digest.into()
    }
}

// Signed transaction.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl Hashable for SignedTransaction{
    fn hash(&self) -> H256 {
        let t_bytes = bincode::serialize(&self).unwrap();
        let t_digest = ring::digest::digest(&ring::digest::SHA256, &t_bytes);
        t_digest.into()
    }
}

impl SignedTransaction {
    pub fn is_valid(&self, state: &State) -> bool {
        let address: H160 = ring::digest::digest(&ring::digest::SHA256, self.public_key.as_ref()).into();
        if self.is_erasable(state) {
            return false;
        }
        if let Some(peer_state) = state.account_state.get(&address) {
            if self.transaction.account_nonce != (peer_state.nonce + 1) {
                return false
            }
        }
        return true;
    }

    pub fn is_erasable(&self, state: &State) -> bool {
        let address: H160 = ring::digest::digest(&ring::digest::SHA256, self.public_key.as_ref()).into();
        let public_key = UnparsedPublicKey::new(&ED25519, self.public_key.clone());
        // verification fails
        if public_key.verify(self.transaction.hash().as_ref(), self.signature.as_ref()).is_err() {
            return true;
        }
        // get the peer state
        if let Some(peer_state) = state.account_state.get(&address) {
            // the nonce is smaller
            if self.transaction.account_nonce <= peer_state.nonce {
                return true;
            }
            // the balance is not enough
            if self.transaction.value > peer_state.balance {
                return true;
            }
        }
        return false;
    }

    pub fn update_state(&self, state: &mut State){
        let address: H160 = ring::digest::digest(&ring::digest::SHA256, self.public_key.as_ref()).into();
        if let Some(peer_state) = state.account_state.get_mut(&address) {
            assert_eq!(peer_state.nonce + 1, self.transaction.account_nonce);
            peer_state.balance -= self.transaction.value;
            peer_state.nonce += 1;
        }
    }
}

    /// Create digital signature of a transaction
    pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
        let t_hash = t.hash();
        key.sign(t_hash.as_ref())  
    }

    /// Verify digital signature of a transaction, using public key instead of secret key
    pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
        let t_hash = t.hash();
        let public_key = UnparsedPublicKey::new(&ED25519, public_key);
        public_key.verify(t_hash.as_ref(), signature.as_ref()).is_ok()
    }

#[cfg(any(test, test_utilities))]
    mod tests {
        use super::*;
        use crate::crypto::key_pair;

        pub fn generate_random_transaction() -> Transaction {
            Default::default()
        }

        #[test]
        fn sign_verify() {
            for _ in 0..20 {
                let t = generate_random_transaction();
                let key = key_pair::random();
                let signature = sign(&t, &key);
                assert!(verify(&t, &(key.public_key()), &signature));
            }
        }
    }
