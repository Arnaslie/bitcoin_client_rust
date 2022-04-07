use super::hash::{Hashable, H256};
use std::collections::{HashMap};
use ring::digest;

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    pub nodes: Vec<H256>,
    pub node_amount: usize,
    pub tree_map: HashMap<usize, usize>,
    pub root_index: usize,
    pub leaf_nodes: usize
}

pub fn print_to_hex(value: &[u8] , bits: usize) { 
    for i in 0..bits {
        print!("{:02x}", value[i]);
    }
    println!();
}

/**
 * This function builds the merkle tree hash map. That is,
 * a key is an index and the value is its parent in the next layer
 * which is also an index. Reusing the value as the key will ultimately
 * return the last index which is the merkle root.
 * 
 * leaf_size (usize): amount of nodes in the bottom layer. will always be
 *                    even because we add an extra block if odd.
 */
pub fn build_tree_map(leaf_size: usize) -> (HashMap<usize, usize>, usize) {
    let mut map: HashMap<usize, usize> = HashMap::new();
    let mut running_node_amount: usize = leaf_size; //represents the index of the first block in the next layer
    let mut nodes_in_layer: usize = leaf_size;
    let mut current_parent: usize = leaf_size;
    let mut current_node: usize = 0;

    //build map until we've reached merkle root
    while nodes_in_layer != 1 {
        //iterate through nodes in a layer
        while (current_node) != (running_node_amount) {
            //connect pairs of blocks to their parents
            map.insert(current_node, current_parent);
            map.insert(current_node+1, current_parent);

            //update indices
            current_node = current_node + 2;
            current_parent = current_parent + 1;
            if current_node == running_node_amount && ((nodes_in_layer / 2) % 2 == 1) {
                current_parent = current_parent + 1;
            }
        }

        nodes_in_layer = nodes_in_layer / 2;
        //if next layer has odd amount just add 1 because we add an extra block to make it even
        //BUT don't do it if we've reached the merkle root
        if nodes_in_layer % 2 == 1  && nodes_in_layer != 1 {
            nodes_in_layer = nodes_in_layer + 1;
        }
        running_node_amount = running_node_amount + nodes_in_layer;
    }

    //running node amount will be the total # of blocks so subtract 1 for root index
    return (map, running_node_amount - 1);
}

/**
 * This function takes in an even-sized vector (layer of blocks).
 * It concatenates 2 consecutive blocks, hashes it to create
 * a next-layer block, and adds it to a vector. It returns the vector
 * which represents the next layer of blocks. If the input is of size
 * 2 then it will output the merkle root block.
 */
pub fn reduce_layer(old_layer: &[H256], length: usize) -> Vec<H256> {
    let mut new_layer: Vec<H256> = Vec::new();
    let mut concat_hash: [u8; 64] = [0; 64];
    let mut index: usize = 0;
    if length == 2 {
        let (left, right) = concat_hash.split_at_mut(32);
        left.copy_from_slice(&old_layer[0].as_ref());
        right.copy_from_slice(&old_layer[1].as_ref());

        //https://stackoverflow.com/questions/67661782/rust-collect-result-of-chain-into-byte-array
        let mut new_hash: [u8; 32] = [0; 32];
        new_hash.copy_from_slice(&ring::digest::digest(&digest::SHA256, &concat_hash).as_ref()[0..32]);
        
        // print!("Hashed Concatenation: ");
        // print_to_hex(&new_hash, 32);
        
        new_layer.push(H256::from(new_hash));
        return new_layer;
    }

    //concatenate pairs of elements until layer is done
    while (index + 2) != (old_layer.len() + 2) {
        let (left, right) = concat_hash.split_at_mut(32);
        left.copy_from_slice(&old_layer[index].as_ref());
        right.copy_from_slice(&old_layer[index+1].as_ref());

        //https://stackoverflow.com/questions/67661782/rust-collect-result-of-chain-into-byte-array
        let mut new_hash: [u8; 32] = [0; 32];
        new_hash.copy_from_slice(&ring::digest::digest(&digest::SHA256, &concat_hash).as_ref()[0..32]);
        
        // print!("Hashed Concatenation: ");
        // print_to_hex(&new_hash, 32);
        
        new_layer.push(H256::from(new_hash));

        index = index + 2;
    }

    return new_layer;
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        let mut merkle_tree: Vec<H256> = Vec::new();
        let mut block_maps: HashMap<usize, usize> = HashMap::new();
        if data.len() == 0 {
            return MerkleTree {nodes: merkle_tree, node_amount: 0, tree_map: block_maps, root_index: 0, leaf_nodes: 0};
        }

        //create hashes of each element in the slice
        for element in data.iter() {
            // println!("Hash: {}", element.hash());
            merkle_tree.push(element.hash());
        }
        let leaf_nodes = merkle_tree.len(); // keep original amount of leaves in 

        //odd amount, duplicate last element in list
        if merkle_tree.len() % 2 == 1 {
            let last_element = &data[data.len() - 1];
            merkle_tree.push(last_element.hash());
        }

        //leaf_size includes a duplicated block such that it's always even
        let leaf_size = merkle_tree.len();
        //copy hashed first layer
        let mut old_layer: Vec<H256> = merkle_tree.clone();
        let mut new_layer: Vec<H256>;

        //reduce layers until merkle root is created
        loop {
            //reduce a layer to its next layer
            new_layer = reduce_layer(&old_layer, old_layer.len());
            let new_layer_size = new_layer.len();

            //need to make odd-lengthed layers even-lengthed EXCEPT when we found the merkle root
            if new_layer_size % 2 == 1 && new_layer_size != 1 {
                let last_element: &H256 = &new_layer[new_layer.len() - 1];
                new_layer.push(*last_element);
            }

            //update old layer
            old_layer = new_layer.clone();

            //append the new layer to original merkle tree
            merkle_tree.append(&mut new_layer);
            
            //reduce and append until we've reached the merkle root
            if new_layer_size == 1 {
                break;
            }
        }

        //build hash map of block indices to their parents; returns the map + root index
        let tuple: (HashMap<usize, usize>, usize) = build_tree_map(leaf_size);
        block_maps = tuple.0;

        let node_amount = merkle_tree.len();
        return MerkleTree {
            nodes: merkle_tree,
            node_amount: node_amount,
            tree_map: block_maps,
            root_index: tuple.1,
            leaf_nodes: leaf_nodes
        };
    }

    pub fn root(&self) -> H256 {
        if self.node_amount == 0 {
            return H256::from([0; 32]);
        }
        return *self.nodes.last().unwrap();
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut proof_vector: Vec<H256> = Vec::new();
        if index >= self.leaf_nodes {
            return proof_vector;
        }

        // println!("BEFORE PRINTING MAP");
        // for (key, value) in (&self.tree_map).into_iter() {
        //     println!("{} -> {}", key, value);
        // }
        // println!("AFTER PRINTING MAP");
        // println!("ROOT INDEX: {}", self.root_index);

        if index % 2 == 0 {
            proof_vector.push(*&self.nodes[index+1]);
        } else {
            proof_vector.push(*&self.nodes[index-1]);
        }
        let mut new_key: &usize = self.tree_map.get(&index).unwrap();
        while new_key != &self.root_index {
            if new_key % 2 == 0 {
                proof_vector.push(*&self.nodes[new_key+1]);
            } else {
                proof_vector.push(*&self.nodes[new_key-1]);
            }
            new_key = self.tree_map.get(new_key).unwrap();
        }

        return proof_vector;
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
/* This function takes in a root, hashed datum, vector of hashes(proof), index, and leaf size
it goes through, starting at the index, the merkle tree going up and verifying with sibling hashes
to confirm that the given data is in the tree
Ouputs true/false depending on it the given root and final hash match
 */
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    let mut is_verified = false;
    let mut hashed = *datum;
    let mut sibling_hash: H256 = *datum;
    let mut concat_hash: [u8; 64] = [0; 64];
    let mut index_ = index;
    let mut leaf_size_ = leaf_size;
    if leaf_size % 2 == 1 { leaf_size_ += 1;}
    let _tuple: (HashMap<usize, usize>, usize) = build_tree_map(leaf_size_);
    let hash_tree = _tuple.0;

    if index >= leaf_size {
        return false;
    }

    let mut new_hash: [u8; 32] = [0; 32];
    let mut i: usize = 0;
    while i < proof.len() { // Checking the opposing index proof to hash with and form parent hash
        if index_ % 2 == 0 {
            let (left, right) = concat_hash.split_at_mut(32);
            left.copy_from_slice(hashed.as_ref());
            right.copy_from_slice(proof[i].as_ref());
            new_hash.copy_from_slice(&ring::digest::digest(&digest::SHA256, &concat_hash).as_ref()[0..32]);
            sibling_hash = H256::from(new_hash);
        }
        else if index_ % 2 == 1 {
            let (left, right) = concat_hash.split_at_mut(32);
            left.copy_from_slice(proof[i].as_ref());
            right.copy_from_slice(hashed.as_ref());
            new_hash.copy_from_slice(&ring::digest::digest(&digest::SHA256, &concat_hash).as_ref()[0..32]);
            sibling_hash = H256::from(new_hash);
        }
        index_ = *hash_tree.get(&index_).unwrap(); // update index & hash
        hashed = sibling_hash;
        i += 1;
    }
    if sibling_hash == *root { is_verified = true; } // check if verified hash equates to root hash
    return is_verified;
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use ntest::assert_false;

    use crate::types::hash::H256;
    use super::*;
    use hex_literal::hex;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    macro_rules! gen_merkle_tree_data_5 {
        () => {{
            vec![
                (hex!("d424382d2b06092e6c7e2d97a6b206f016c00eadde93658ea7dd45be6f54ef4d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),

                (hex!("d424382d2b06092e6c7e2d97a6b206f016c00eadde93658ea7dd45be6f54ef4d")).into(),
                (hex!("a529f216c18a74668a7681aa9f59b59551bcd9f4c7c9f4dd88b7b07fcff5cc65")).into(),

                (hex!("59fbe39cadc2188730d2ae81cfa3b03221b6819980f9f2caac8ba353d5ad1a62")).into(),
            ]
        }};
    }

    macro_rules! gen_merkle_tree_data_6 {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),

                (hex!("1a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("1101010101010101010101010101010101010101010101010101010101010202")).into(),

                (hex!("2a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("2101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    macro_rules! gen_merkle_tree_data_8 {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),

                (hex!("1a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("1101010101010101010101010101010101010101010101010101010101010202")).into(),

                (hex!("2a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("2101010101010101010101010101010101010101010101010101010101010202")).into(),

                (hex!("3a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("3101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        let input_data_5: Vec<H256> = gen_merkle_tree_data_5!();
        let merkle_tree_5 = MerkleTree::new(&input_data_5);
        let root_5 = merkle_tree_5.root();
        assert_eq!(
            root_5,
            (hex!("bfebc21f187398781cda77b9edacc6872da485c1307260905ac08c4b1e6c7b43")).into()
        );

        let input_data_6: Vec<H256> = gen_merkle_tree_data_6!();
        let merkle_tree_6 = MerkleTree::new(&input_data_6);
        let root_6 = merkle_tree_6.root();
        assert_eq!(
            root_6,
            (hex!("1ce938947f5deeb83731656790f382a1942cefee29b3baeb9aafbc20d59111ce")).into()
        );

        let input_data_8: Vec<H256> = gen_merkle_tree_data_8!();
        let merkle_tree_8 = MerkleTree::new(&input_data_8);
        let root_8 = merkle_tree_8.root();
        assert_eq!(
            root_8,
            (hex!("efcfc1c376f933a3e348ddd4891b63ec719eb68b5d7f8c8ab72f7fb72b9f96f9")).into()
        );
    }

    #[test]
    fn merkle_proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        proof = merkle_tree.proof(1);
        assert_eq!(proof,
                   vec![hex!("b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0").into()]
        );

        proof = merkle_tree.proof(2);
        assert_eq!(proof, Vec::new());
    }

    #[test]
    fn merkle_proof_5() {
        let input_data: Vec<H256> = gen_merkle_tree_data_5!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![
                        hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into(),
                        hex!("267a9277704bf636d5342c3226c1dd9ac7e73c98dfb631ab6e93846b2aeacd42").into(),
                        hex!("feebcb7417406640e0438002cde6e3d228eb0ad7f78243a64c335dfb402e0391").into(),
                    ]
        );

        proof = merkle_tree.proof(1);
        assert_eq!(proof,
                    vec![
                        hex!("922d88de341be512ee300a36672e97d75a0e3e1cd44a1f38624fc979b64992d4").into(),
                        hex!("267a9277704bf636d5342c3226c1dd9ac7e73c98dfb631ab6e93846b2aeacd42").into(),
                        hex!("feebcb7417406640e0438002cde6e3d228eb0ad7f78243a64c335dfb402e0391").into(),
                    ]
        );

        proof = merkle_tree.proof(2);
        assert_eq!(proof,
                    vec![
                        hex!("899e021d15e04d8e1dc1211089e24ac8c9800e4d677a07eaeafcf6472ed7b3ae").into(),
                        hex!("e632f51e22e87898469269a7c0946cee59b09a2d68d119eca9e4395cf3b25944").into(),
                        hex!("feebcb7417406640e0438002cde6e3d228eb0ad7f78243a64c335dfb402e0391").into(),
                    ]
        );

        proof = merkle_tree.proof(3);
        assert_eq!(proof,
                    vec![
                        hex!("922d88de341be512ee300a36672e97d75a0e3e1cd44a1f38624fc979b64992d4").into(),
                        hex!("e632f51e22e87898469269a7c0946cee59b09a2d68d119eca9e4395cf3b25944").into(),
                        hex!("feebcb7417406640e0438002cde6e3d228eb0ad7f78243a64c335dfb402e0391").into(),
                    ]
        );
        
        proof = merkle_tree.proof(4);
        assert_eq!(proof,
                    vec![
                        hex!("7fdab699d4d2563ad9b1d38b6d9a1fc313b6b6851960c49d3a27684ef3fc3bbd").into(),
                        hex!("646911badbe585e635f69d94482016a2006e909052a76f63bbc6f006fe71ea72").into(),
                        hex!("7d9909c8224470bc940bbd14c4bc22dd9c823f59e2cc770635c2140de2d1999a").into(),
                    ]
        );

        proof = merkle_tree.proof(5);
        assert_eq!(proof, Vec::new());
    }

    #[test]
    fn merkle_proof_6() {
        let input_data: Vec<H256> = gen_merkle_tree_data_6!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof = merkle_tree.proof(0);
        assert_eq!(proof,
                    vec![
                        hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into(),
                        hex!("853e37decf5a6f790e2fffc0b099d0ffea6c6661c7cd827fe3937d094c443921").into(),
                        hex!("66f66ea1d27ba51269f64bff4af8a2432cc03b0d8204c3b0866e83df2b7e656a").into(),
                    ]
        );

        proof = merkle_tree.proof(1);
        assert_eq!(proof,
                    vec![
                        hex!("b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0").into(),
                        hex!("853e37decf5a6f790e2fffc0b099d0ffea6c6661c7cd827fe3937d094c443921").into(),
                        hex!("66f66ea1d27ba51269f64bff4af8a2432cc03b0d8204c3b0866e83df2b7e656a").into(),
                    ]
        );

        proof = merkle_tree.proof(2);
        assert_eq!(proof,
                    vec![
                        hex!("b3428fcfa18be5063fb9d3b330ec730529abbb22d19f2a4a33084ca57457a043").into(),
                        hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into(),
                        hex!("66f66ea1d27ba51269f64bff4af8a2432cc03b0d8204c3b0866e83df2b7e656a").into(),
                    ]
        );

        proof = merkle_tree.proof(3);
        assert_eq!(proof,
                    vec![
                        hex!("d424382d2b06092e6c7e2d97a6b206f016c00eadde93658ea7dd45be6f54ef4d").into(),
                        hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into(),
                        hex!("66f66ea1d27ba51269f64bff4af8a2432cc03b0d8204c3b0866e83df2b7e656a").into(),
                    ]
        );

        proof = merkle_tree.proof(4);
        assert_eq!(proof,
                    vec![
                        hex!("a529f216c18a74668a7681aa9f59b59551bcd9f4c7c9f4dd88b7b07fcff5cc65").into(),
                        hex!("e4be091a66883ca3116bfa577d099854db54e33481751eb7f8788bababf9e768").into(),
                        hex!("68e28eca86d3185342f9b91c0f81acd68974d52aec407e535fa7a68c0555c7d5").into(),
                    ]
        );

        proof = merkle_tree.proof(5);
        assert_eq!(proof,
                    vec![
                        hex!("60f691b482e6b5b45a47e0bd39b613b28011bfc40e7a91ef39de2865f9330265").into(),
                        hex!("e4be091a66883ca3116bfa577d099854db54e33481751eb7f8788bababf9e768").into(),
                        hex!("68e28eca86d3185342f9b91c0f81acd68974d52aec407e535fa7a68c0555c7d5").into(),
                    ]
        );

        proof = merkle_tree.proof(6);
        assert_eq!(proof, Vec::new());
    }

    #[test]
    fn merkle_proof_8() {
        let input_data: Vec<H256> = gen_merkle_tree_data_8!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof = merkle_tree.proof(0);
        assert_eq!(proof,
                    vec![
                        hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into(),
                        hex!("853e37decf5a6f790e2fffc0b099d0ffea6c6661c7cd827fe3937d094c443921").into(),
                        hex!("0bf69f0d0c7ff2d4953163ce7d2b5507ef1eda217bccea997b479090d71aeceb").into(),
                    ]
        );

        proof = merkle_tree.proof(1);
        assert_eq!(proof,
                    vec![
                        hex!("b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0").into(),
                        hex!("853e37decf5a6f790e2fffc0b099d0ffea6c6661c7cd827fe3937d094c443921").into(),
                        hex!("0bf69f0d0c7ff2d4953163ce7d2b5507ef1eda217bccea997b479090d71aeceb").into(),
                    ]
        );

        proof = merkle_tree.proof(2);
        assert_eq!(proof,
                    vec![
                        hex!("b3428fcfa18be5063fb9d3b330ec730529abbb22d19f2a4a33084ca57457a043").into(),
                        hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into(),
                        hex!("0bf69f0d0c7ff2d4953163ce7d2b5507ef1eda217bccea997b479090d71aeceb").into(),
                    ]
        );

        proof = merkle_tree.proof(3);
        assert_eq!(proof,
                    vec![
                        hex!("d424382d2b06092e6c7e2d97a6b206f016c00eadde93658ea7dd45be6f54ef4d").into(),
                        hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into(),
                        hex!("0bf69f0d0c7ff2d4953163ce7d2b5507ef1eda217bccea997b479090d71aeceb").into(),
                    ]
        );

        proof = merkle_tree.proof(4);
        assert_eq!(proof,
                    vec![
                        hex!("a529f216c18a74668a7681aa9f59b59551bcd9f4c7c9f4dd88b7b07fcff5cc65").into(),
                        hex!("8236f1aedb026dbd13237693df799436768d5546dc8a18668ec2e51f91dc90c8").into(),
                        hex!("68e28eca86d3185342f9b91c0f81acd68974d52aec407e535fa7a68c0555c7d5").into(),
                    ]
        );

        proof = merkle_tree.proof(5);
        assert_eq!(proof,
                    vec![
                        hex!("60f691b482e6b5b45a47e0bd39b613b28011bfc40e7a91ef39de2865f9330265").into(),
                        hex!("8236f1aedb026dbd13237693df799436768d5546dc8a18668ec2e51f91dc90c8").into(),
                        hex!("68e28eca86d3185342f9b91c0f81acd68974d52aec407e535fa7a68c0555c7d5").into(),
                    ]
        );

        proof = merkle_tree.proof(6);
        assert_eq!(proof,
                    vec![
                        hex!("59fbe39cadc2188730d2ae81cfa3b03221b6819980f9f2caac8ba353d5ad1a62").into(),
                        hex!("e4be091a66883ca3116bfa577d099854db54e33481751eb7f8788bababf9e768").into(),
                        hex!("68e28eca86d3185342f9b91c0f81acd68974d52aec407e535fa7a68c0555c7d5").into(),
                    ]
        );

        proof = merkle_tree.proof(7);
        assert_eq!(proof,
                   vec![
                       hex!("fbe1c195012727ce75535bce245fe6211998b180ab2b91acf03064c4e043fc46").into(),
                       hex!("e4be091a66883ca3116bfa577d099854db54e33481751eb7f8788bababf9e768").into(),
                       hex!("68e28eca86d3185342f9b91c0f81acd68974d52aec407e535fa7a68c0555c7d5").into(),
                    ]
        );

        proof = merkle_tree.proof(8);
        assert_eq!(proof, Vec::new());
    }

    #[test]
    fn merkle_verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
        
        proof = merkle_tree.proof(1);
        assert!(verify(&merkle_tree.root(), &input_data[1].hash(), &proof, 1, input_data.len()));

        assert_false!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 8, input_data.len()));
    }

    #[test]
    fn merkle_verifying_5() {
        let input_data: Vec<H256> = gen_merkle_tree_data_5!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));

        proof = merkle_tree.proof(1);
        assert!(verify(&merkle_tree.root(), &input_data[1].hash(), &proof, 1, input_data.len()));
        proof = merkle_tree.proof(2);
        assert!(verify(&merkle_tree.root(), &input_data[2].hash(), &proof, 2, input_data.len()));
        proof = merkle_tree.proof(3);
        assert!(verify(&merkle_tree.root(), &input_data[3].hash(), &proof, 3, input_data.len()));
        proof = merkle_tree.proof(4);
        assert!(verify(&merkle_tree.root(), &input_data[4].hash(), &proof, 4, input_data.len()));

        assert_false!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 8, input_data.len()));
    }

    #[test]
    fn merkle_verifying_6() {
        let input_data: Vec<H256> = gen_merkle_tree_data_6!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));

        proof = merkle_tree.proof(1);
        assert!(verify(&merkle_tree.root(), &input_data[1].hash(), &proof, 1, input_data.len()));
        proof = merkle_tree.proof(2);
        assert!(verify(&merkle_tree.root(), &input_data[2].hash(), &proof, 2, input_data.len()));
        proof = merkle_tree.proof(3);
        assert!(verify(&merkle_tree.root(), &input_data[3].hash(), &proof, 3, input_data.len()));
        proof = merkle_tree.proof(4);
        assert!(verify(&merkle_tree.root(), &input_data[4].hash(), &proof, 4, input_data.len()));
        proof = merkle_tree.proof(5);
        assert!(verify(&merkle_tree.root(), &input_data[5].hash(), &proof, 5, input_data.len()));

        assert_false!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 8, input_data.len()));
    }
    
    #[test]
    fn merkle_verifying_8() {
        let input_data: Vec<H256> = gen_merkle_tree_data_8!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));

        proof = merkle_tree.proof(1);
        assert!(verify(&merkle_tree.root(), &input_data[1].hash(), &proof, 1, input_data.len()));
        proof = merkle_tree.proof(7);
        assert!(verify(&merkle_tree.root(), &input_data[7].hash(), &proof, 7, input_data.len()));
        proof = merkle_tree.proof(6);
        assert!(verify(&merkle_tree.root(), &input_data[6].hash(), &proof, 6, input_data.len()));
        proof = merkle_tree.proof(2);
        assert!(verify(&merkle_tree.root(), &input_data[2].hash(), &proof, 2, input_data.len()));
        proof = merkle_tree.proof(3);
        assert!(verify(&merkle_tree.root(), &input_data[3].hash(), &proof, 3, input_data.len()));
        proof = merkle_tree.proof(4);
        assert!(verify(&merkle_tree.root(), &input_data[4].hash(), &proof, 4, input_data.len()));
        proof = merkle_tree.proof(5);
        assert!(verify(&merkle_tree.root(), &input_data[5].hash(), &proof, 5, input_data.len()));

        assert_false!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 15, input_data.len()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST