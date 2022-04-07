use serde::{Serialize, Deserialize};
use crate::types::hash::{H256, Hashable};
use std::collections::HashMap;
use super::address::Address;
use super::transaction::{SignedTransaction};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
}

pub struct BlockState {
    //block hash -> block state (account address -> (account nonce, account balance))
    pub block_state_map: HashMap<H256, HashMap<Address, (u32, u32)>>
}

impl BlockState {
    pub fn new() -> Self {
        return BlockState {
            block_state_map: HashMap::<H256, HashMap<Address, (u32, u32)>>::new()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub parent: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128,
    pub merkle_root: H256
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Content {
    pub data: Vec<SignedTransaction>
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        let header = self.get_header();
        return header.hash();
    }
}

impl Hashable for Header {
    fn hash(&self) -> H256 {
        let serialized = bincode::serialize(self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &serialized).into()
    }
}

impl Block {
    pub fn get_header(&self) -> Header {
        return self.header.clone();
    }

    pub fn get_content(&self) -> Content {
        return self.content.clone();
    }

    //ORIGINALLY DEFINED
    pub fn get_parent(&self) -> H256 {
        return self.header.parent.clone();
    }

    //ORIGINALLY DEFINED
    pub fn get_difficulty(&self) -> H256 {
        return self.header.difficulty.clone();
    }

    pub fn get_nonce(&self) -> u32 {
        return self.header.nonce.clone();
    }

    pub fn get_timestamp(&self) -> u128 {
        return self.header.timestamp.clone();
    }

    pub fn get_merkle_root(&self) -> H256 {
        return self.header.merkle_root.clone();
    }
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_block(parent: &H256) -> Block {
    use std::time::{SystemTime, UNIX_EPOCH};
    use crate::types::merkle::MerkleTree;
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let start = SystemTime::now();
    let timestamp = start.duration_since(UNIX_EPOCH).expect("Time went backwards").as_micros();
    let merkle_tree = MerkleTree::new(&Vec::<SignedTransaction>::new());
    let difficulty = H256::from(rng.gen::<[u8; 32]>());
    let nonce = rng.gen::<u32>();

    let header = Header {
        parent: *parent,
        nonce: nonce,
        difficulty: difficulty,
        timestamp: timestamp,
        merkle_root: merkle_tree.root()
    };
    
    let content = Content {
        data: Vec::<SignedTransaction>::new()
    };

    let new_block = Block {
        header: header,
        content: content
    };

    return new_block;
}
