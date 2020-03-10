use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters, UnparsedPublicKey, ED25519};
use crate::crypto::hash::{H256, Hashable};
use crate::crypto::address::{H160};

// Account based model transaction (Ethereum).
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    recipient_address: H160,
    value: u64,
    account_nonce: H256,
}

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
        Transaction { 
            recipient_address: rand::random::<[u8; 20]>().into(),
            value: rand::random(),
            account_nonce: rand::random::<[u8; 32]>().into(),
        }
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
