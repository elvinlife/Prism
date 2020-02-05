use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters, UnparsedPublicKey, ED25519};
use rand::{Rng, distributions::Alphanumeric};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Transaction {
    input: String,
    output: String,
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let t_bytes = bincode::serialize(t).unwrap();
    let t_digest = ring::digest::digest(&ring::digest::SHA256, &t_bytes);
    key.sign(t_digest.as_ref())  
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
    let t_bytes = bincode::serialize(t).unwrap();
    let t_digest = ring::digest::digest(&ring::digest::SHA256, &t_bytes);
    let public_key = UnparsedPublicKey::new(&ED25519, public_key);
    public_key.verify(t_digest.as_ref(), signature.as_ref()).is_ok()
}

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::crypto::key_pair;

    pub fn generate_random_transaction() -> Transaction {
        let rand_input = rand::thread_rng().sample_iter(&Alphanumeric).take(10).collect::<String>();
        let rand_output = rand::thread_rng().sample_iter(&Alphanumeric).take(10).collect::<String>();
        Transaction { input : rand_input, output: rand_output }
    }

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &(key.public_key()), &signature));
    }
}
