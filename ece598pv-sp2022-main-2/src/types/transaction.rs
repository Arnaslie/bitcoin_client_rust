use std::collections::HashMap;

use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, self};

use super::address::Address;
use super::hash::{H256, Hashable};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub sender: Address,
    pub account_nonce: u32,
    pub receiver: Address,
    pub value: u32
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let serialized = bincode::serialize(self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &serialized).into()
    }
}

pub struct ICO {
    //account address -> (account nonce, account balance)
    pub state: HashMap<Address, (u32, u32)>
}

impl ICO {
    pub fn new(pubkey: &[u8]) -> Self {
        //Do Initial Coin Offering (ICO); hardcode an account using the given pubkey
        let account_address = Address::from_public_key_bytes(pubkey);
        let balance = 1000000;
        let mut map = HashMap::new();
        map.insert(account_address, (0, balance));
        return ICO {
            state: map
        }
    }
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let serialized_transaction: Vec<u8> = bincode::serialize(&t).unwrap();
    return key.sign(&serialized_transaction);
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &[u8], signature: &[u8]) -> bool {
    let serialized_transaction: Vec<u8> = bincode::serialize(&t).unwrap();
    let pub_key = signature::UnparsedPublicKey::new(&signature::ED25519, public_key);
    return pub_key.verify(&serialized_transaction, signature).is_ok();
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_transaction() -> Transaction {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_value: u32 = rng.gen::<u32>();
    let random_receiver: [u8; 20] = rng.gen::<[u8; 20]>();
    let random_sender: [u8; 20] = rng.gen::<[u8; 20]>();
    return Transaction {sender: Address::from(random_sender), receiver: Address::from(random_receiver), value: random_value, account_nonce:0};
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::key_pair;
    use ring::signature::KeyPair;


    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, key.public_key().as_ref(), signature.as_ref()));
    }
    #[test]
    fn sign_verify_two() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        let key_2 = key_pair::random();
        let t_2 = generate_random_transaction();
        assert!(!verify(&t_2, key.public_key().as_ref(), signature.as_ref()));
        assert!(!verify(&t, key_2.public_key().as_ref(), signature.as_ref()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST