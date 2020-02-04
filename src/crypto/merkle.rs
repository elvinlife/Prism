use super::hash::{Hashable, H256};
use std::vec::Vec;

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    tree: Vec<H256>,
    valid: Vec<bool>,
    sz: usize,
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        let mut _sz = 1;
        while _sz < data.len(){
            _sz = _sz << 1;
        }

        let mut _tree: Vec<H256> = Vec::<H256>::new();
        _tree.resize(2*_sz-1,Default::default());
        let mut _valid: Vec<bool> = Vec::<bool>::new();
        _valid.resize(2*_sz-1,false);

        for i in 0..data.len(){
            _tree[i+_sz-1] = data[i].hash();
            _valid[i+_sz-1] = true;
        }

        let save_sz = _sz;
        while _sz > 1{
            let mut i = 0;
            while i < _sz {
                let l_idx = _sz - 1 + i;
                let r_idx = l_idx + 1;
                let p_idx = (l_idx - 1) >> 1;

                let mut buf : Vec<u8> = Vec::<u8>::new();

                if !_valid[l_idx]{
                    break;
                }
                else if _valid[l_idx] && !_valid[r_idx]{
                    _tree[r_idx] = _tree[l_idx];
                    buf.extend_from_slice(_tree[l_idx].as_ref()); 
                    buf.extend_from_slice(_tree[r_idx].as_ref());
                    _tree[p_idx] = ring::digest::digest(&ring::digest::SHA256, &buf).into();
                    _valid[p_idx] = true;
                }
                else{
                    buf.extend_from_slice(_tree[l_idx].as_ref()); 
                    buf.extend_from_slice(_tree[r_idx].as_ref());
                    _tree[p_idx] = ring::digest::digest(&ring::digest::SHA256, &buf).into();
                    _valid[p_idx] = true;
                } 

                i += 2;
            }
            _sz  = _sz >> 1;
        }

        MerkleTree{
            tree: _tree,
            valid: _valid,
            sz: save_sz,
        }

    }

    pub fn root(&self) -> H256 {
        self.tree[0]
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut proof : Vec<H256> = Vec::<H256>::new();
        let mut idx = self.sz - 1 + index;
        if idx < 2*self.sz - 1 && self.valid[idx]{
            while idx > 0{
                let p_idx = (idx - 1) >> 1;
                let s_idx = if idx % 2 == 1{
                    idx + 1
                }
                else{
                    idx - 1
                };
                proof.push(self.tree[s_idx]);
                idx = p_idx;
            }
        } 
        proof 
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    let mut _sz = 1;
    let mut cnt = 0;
    while _sz < leaf_size { _sz = _sz << 1; cnt += 1; }
    
    if index >= leaf_size || proof.len() != cnt {
        false
    } 
    else{
        let mut idx = index;
        let mut curr : H256 = *datum;

        for hash in proof{
            let mut buf : Vec<u8> = Vec::<u8>::new();
            if idx % 2 == 0{
                buf.extend_from_slice(curr.as_ref());
                buf.extend_from_slice(hash.as_ref());
                curr = ring::digest::digest(&ring::digest::SHA256,&buf).into(); 
            }
            else{
                buf.extend_from_slice(hash.as_ref());
                buf.extend_from_slice(curr.as_ref());
                curr = ring::digest::digest(&ring::digest::SHA256,&buf).into();
            }
            idx = idx >> 1;
        }
    
        *root == curr 
    }
    
}

#[cfg(test)]
mod tests {
    use crate::crypto::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            //vec![
            //    (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
            //    (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            //]
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010203")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010204")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010205")).into(),

            ]
        }};
    }

    #[test]
    fn root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            //(hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
            (hex!("ef823a0327b78067ec81340c1513c70bb76871b39ca2ac5072885e167b835b22")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   //vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
                    vec![
                        hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into(),
                        hex!("b818366af651c9c84b6a09df4927821b2b33c9e4abfd0e03d4be882cb609e504").into(),
                        hex!("38af33ff1e555412e0c80ad03cde61a41ef95d7928c39d436da2ee2a834f252b").into(),
                   ]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(3);
        assert!(verify(&merkle_tree.root(), &input_data[3].hash(), &proof, 3, input_data.len()));
    }
}
