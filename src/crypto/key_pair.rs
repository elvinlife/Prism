use ring::rand;
use ring::signature::Ed25519KeyPair;
use ring::test::rand::FixedByteRandom;

/// Generate a random key pair.
pub fn random() -> Ed25519KeyPair {
    let rng = rand::SystemRandom::new();
    let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref().into()).unwrap()
}

pub fn frombyte(i: u8) -> Ed25519KeyPair {
    let byterandom = FixedByteRandom {
        byte: i,
    };
    let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&byterandom).unwrap();
    Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref().into()).unwrap()
}
