use super::hash::{Hashable, H256};
use std::vec::Vec;

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    tree: Vec<H256>,    // Vector of tree nodes.
    valid: Vec<bool>,   // Vector of flags indicating whether index in tree[] corresponds to valid node.
    sz: usize,          // Next greatest power of 2 of the leaf size.
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        // Find the next greatest power of 2 of the leaf size.
        let mut _sz = 1;
        while _sz < data.len(){
            _sz = _sz << 1;
        }

        // Initialize tree[] and valid[] to have 2*sz-1 elements.
        let mut _tree: Vec<H256> = Vec::<H256>::new();
        _tree.resize(2*_sz-1,Default::default());
        let mut _valid: Vec<bool> = Vec::<bool>::new();
        _valid.resize(2*_sz-1,false);

        // Copy the input data to the last level of the tree[].
        for i in 0..data.len(){
            _tree[i+_sz-1] = data[i].hash();
            _valid[i+_sz-1] = true;
        }

        // Construct the tree[] level by level from leaf up to the root.
        let save_sz = _sz;
        while _sz > 1{                                      // While not at level 0 (the root)
            let mut i = 0;                                  // Let i be the current node in the level.
            while i < _sz {                                 // Continue until you reach the end of the level.

                let l_idx = _sz - 1 + i;                    // Index of i in tree[].
                let r_idx = l_idx + 1;                      // Index of right sibling of i in tree[].
                let p_idx = (l_idx - 1) >> 1;               // Index of parent of i in tree[].

                let mut buf : Vec<u8> = Vec::<u8>::new();

                if !_valid[l_idx]{                          // If we reached the end of the level, go to next level.
                    break;
                }
                else if _valid[l_idx] && !_valid[r_idx]{    // Otherwise, if current node is valid but right sibling is invalid, copy current node to right sibling before filling parent.
                    _tree[r_idx] = _tree[l_idx];
                    buf.extend_from_slice(_tree[l_idx].as_ref()); 
                    buf.extend_from_slice(_tree[r_idx].as_ref());
                    _tree[p_idx] = ring::digest::digest(&ring::digest::SHA256, &buf).into();
                    _valid[p_idx] = true;
                }
                else{                                       // Otherwise, fill parent hash with hash of current node and its right sibling.
                    buf.extend_from_slice(_tree[l_idx].as_ref()); 
                    buf.extend_from_slice(_tree[r_idx].as_ref());
                    _tree[p_idx] = ring::digest::digest(&ring::digest::SHA256, &buf).into();
                    _valid[p_idx] = true;
                } 

                i += 2;                                     // Advance current node past its right sibling.
            }
            _sz  = _sz >> 1;                                // Move to next level.
        }

        // Return the constructed tree.
        MerkleTree{
            tree: _tree,
            valid: _valid,
            sz: save_sz,
        }

    }

    pub fn root(&self) -> H256 {
        self.tree[0]                                        // Root of tree is at index 0.
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut proof : Vec<H256> = Vec::<H256>::new();

        let mut idx = self.sz - 1 + index;                 // Get index of leaf in the tree[].

        if idx < 2*self.sz - 1 && self.valid[idx]{         // Make sure this is a valid leaf.

            while idx > 0{                                 // Construct the proof from bottom up until we reach root.
                let p_idx = (idx - 1) >> 1;                // Index of parent.
                let s_idx = if idx % 2 == 1{               // Index of sibling, which depends on whether current node is a left or right child of its parent.
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
    while _sz < leaf_size { _sz = _sz << 1; cnt += 1; }            // Given leaf_size, we expect the proof to have length cnt.
    
    if index >= leaf_size || proof.len() != cnt {                  // If either invalid index or proof length != cnt, prematurely abort.
        false
    } 
    else{
        let mut idx = index;
        let mut curr : H256 = *datum;

        for hash in proof{                                        // Do the proof.
            let mut buf : Vec<u8> = Vec::<u8>::new();
            if idx % 2 == 0{                                      // If the current index is even, we know it is the left child of its parent.
                buf.extend_from_slice(curr.as_ref());
                buf.extend_from_slice(hash.as_ref());
                curr = ring::digest::digest(&ring::digest::SHA256,&buf).into(); 
            }
            else{
                buf.extend_from_slice(hash.as_ref());             // If current index is odd, it is right child of parent.
                buf.extend_from_slice(curr.as_ref());
                curr = ring::digest::digest(&ring::digest::SHA256,&buf).into();
            }
            idx = idx >> 1;
        }
    
        *root == curr                                             // Compare final value with root.
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
        for i in 0..input_data.len(){
            let proof = merkle_tree.proof(i);
            assert!(verify(&merkle_tree.root(), &input_data[i].hash(), &proof, i, input_data.len()));
        }
        for i in input_data.len()-1..=0{
            let proof = merkle_tree.proof(input_data.len()-1-i);
            assert!(!verify(&merkle_tree.root(), &input_data[i].hash(), &proof, input_data.len()-1-i, input_data.len()));
        }
        
    }
}
