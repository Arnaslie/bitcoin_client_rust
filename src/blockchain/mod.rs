use std::collections::HashMap;

use crate::types::hash::{H256, Hashable};
use super::types::block::{Block, Content, Header};
use super::types::merkle::MerkleTree;
use super::types::transaction::SignedTransaction;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;

pub static DIFFICULTY: [u8; 32] = [0, 0, 64, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];

pub struct Blockchain {
    //map a block's hash to a tuple of (the block itself, height in blockchain)
    pub block_map: HashMap<H256, (Block, u32)>,
    pub tip: H256,
    //each block's height will be stored too but store overall height for clarity
    pub height: u32
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let genesis_parent_hash = H256::from([0; 32]);
        let mut rng = rand::thread_rng();
        let start = SystemTime::now();
        //let genesis_timestamp = start.duration_since(UNIX_EPOCH).expect("Time went backwards").as_micros();
        let genesis_timestamp = 0;
        let genesis_merkle_tree = MerkleTree::new(&Vec::<SignedTransaction>::new());
        let genesis_difficulty = DIFFICULTY.into();
        // let genesis_nonce = rng.gen::<u32>();
        let genesis_nonce = 0;
        let genesis_height = 0;

        let genesis_header = Header {
            parent: genesis_parent_hash,
            nonce: genesis_nonce,
            difficulty: genesis_difficulty,
            timestamp: genesis_timestamp,
            merkle_root: genesis_merkle_tree.root()
        };

        let genesis_content = Content {
            data: Vec::<SignedTransaction>::new()
        };
        
        let genesis_block = Block {
            header: genesis_header,
            content: genesis_content
        };

        let mut storage = HashMap::<H256, (Block, u32)>::new();
        storage.insert(genesis_block.clone().hash(), (genesis_block.clone(), genesis_height));

        return Self {
            block_map: storage,
            tip: genesis_block.clone().hash(),
            height: genesis_height
        };
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let new_block_hash = block.hash();
        let new_block_parent_hash = block.get_parent();
        let (new_block_parent, new_block_parent_height) = self.block_map.get(&new_block_parent_hash).unwrap();
        let new_block_height;

        //means we are inserting a new block to the current tip -> UPDATE tip and height
        if new_block_parent_hash == self.tip() {
            let new_height = self.height + 1;
            self.tip = new_block_hash;
            self.height = new_height;
            new_block_height = new_height;
        }
        //means we are forking -> updating tip/height depends on new block's height
        else {
            new_block_height = new_block_parent_height + 1;
            //From MP doc: "You can also store the tip, and update it after inserting a block.
            //If, say, your current tip is hash(B1), and you insert a new block B2: you need to update tip to hash(B2)
            //if and only if the length of chain B2 is *STRICTLY* greater than that of B1."
            //It's strictly greater because if it's (1) less than -> self explanatory or (2) equal to -> we use tie breaking
            //rules of keeping older chain as longest chain.
            if new_block_height > self.height {
                self.height = new_block_height;
                self.tip = new_block_hash;
            }
        }

        self.block_map.insert(new_block_hash, ((*block).clone(), new_block_height));
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        return self.tip;
    }

    /// Get all blocks' hashes of the longest chain, ordered from genesis to the tip
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut chain: Vec<H256> = Vec::<H256>::new();
        let tip: &H256 = &self.tip();
        chain.push(*tip);
        let mut parent_hash: H256 = self.block_map.get(tip).unwrap().0.get_parent();

        //genesis block's parent will be x00..00
        while parent_hash != H256::from([0; 32]) {
            chain.push(parent_hash);
            parent_hash = self.block_map.get(&parent_hash).unwrap().0.get_parent();
        }

        chain.reverse();
        return chain;
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
    }

    #[test]
    //tests chain update behavior for multiple cases
    fn insert_chain_update_behavior() {
        //draw the graph on paper so it's clearer
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block1 = generate_random_block(&genesis_hash);
        let block2 = generate_random_block(&block1.hash());
        let block3 = generate_random_block(&block2.hash());
        let block4 = generate_random_block(&block1.hash());
        let block5 = generate_random_block(&block4.hash());
        let block6 = generate_random_block(&block1.hash());
        let block7 = generate_random_block(&block5.hash());
        let block8 = generate_random_block(&block6.hash());
        let block9 = generate_random_block(&block8.hash());
        let block10 = generate_random_block(&block9.hash());

        //TEST normal insertion behavior where new block's parent is the tip
        blockchain.insert(&block1);
        blockchain.insert(&block2);
        blockchain.insert(&block3);
        let mut vec = Vec::<H256>::from([genesis_hash, block1.hash(), block2.hash(), block3.hash()]);
        assert_eq!(blockchain.tip(), block3.hash());
        assert_eq!(vec, blockchain.all_blocks_in_longest_chain());

        //TEST case where a new chain is created which has same length as current longest chain -> keep current longest chain
        blockchain.insert(&block4);
        blockchain.insert(&block5);
        assert_eq!(blockchain.tip(), block3.hash());
        assert_eq!(vec, blockchain.all_blocks_in_longest_chain());

        //TEST case where new block is inserted to a chain that is shorter than longest chain length
        blockchain.insert(&block6);
        assert_eq!(blockchain.tip(), block3.hash());
        assert_eq!(vec, blockchain.all_blocks_in_longest_chain());

        //TEST case where new block is inserted to a chain that is longer than current longest chain -> switch to new chain
        blockchain.insert(&block7);
        vec = Vec::<H256>::from([genesis_hash, block1.hash(), block4.hash(), block5.hash(), block7.hash()]);
        assert_eq!(blockchain.tip(), block7.hash());
        assert_eq!(vec, blockchain.all_blocks_in_longest_chain());

        //TEST mix of cases as before
        blockchain.insert(&block8);
        assert_eq!(blockchain.tip(), block7.hash());
        assert_eq!(vec, blockchain.all_blocks_in_longest_chain());
        blockchain.insert(&block9);
        assert_eq!(blockchain.tip(), block7.hash());
        assert_eq!(vec, blockchain.all_blocks_in_longest_chain());
        blockchain.insert(&block10);
        vec = Vec::<H256>::from([genesis_hash, block1.hash(), block6.hash(), block8.hash(), block9.hash(), block10.hash()]);
        assert_eq!(blockchain.tip(), block10.hash());
        assert_eq!(vec, blockchain.all_blocks_in_longest_chain());
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST