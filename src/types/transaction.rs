use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters, self};
use rand::{thread_rng, Rng};
use rand::distributions::{Distribution, Standard};
use ring::digest;
use super::address::Address;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    sender: Address,
    receiver: Address,
    value: i32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {

}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let serialized = bincode::serialize(t).unwrap();
    let message = digest::digest(&digest::SHA256, digest::digest(&digest::SHA256, serialized.as_ref()).as_ref());
    let sign = key.sign(message.as_ref());
    return sign;
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &[u8], signature: &[u8]) -> bool {
    let mut key_flag: bool = true;
    let serialized = bincode::serialize(t).unwrap();
    let msg = digest::digest(&digest::SHA256, digest::digest(&digest::SHA256, serialized.as_ref()).as_ref());
    let public_key_check = signature::UnparsedPublicKey::new(&signature::ED25519, public_key.as_ref());
    key_flag = public_key_check.verify(msg.as_ref(), signature.as_ref()).is_ok();
    return key_flag;
}

#[cfg(any(test, test_utilities))]

pub fn generate_random_transaction() -> Transaction {
    use crate::types::key_pair;
    use super::address::Address;
    let mut rng = thread_rng();
    let rnd_value: i32 = rng.gen();

    let key_send = key_pair::random();
    let public_key_send = key_send.public_key();
    let rnd_sender = super::address::Address::from_public_key_bytes(public_key_send.as_ref());

    let key_receive = key_pair::random();
    let public_key_receive = key_receive.public_key();
  
    let rnd_receiver = super::address::Address::from_public_key_bytes(public_key_receive.as_ref());

    let t = Transaction {sender: rnd_sender, receiver: rnd_receiver, value: rnd_value};
    return t;
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