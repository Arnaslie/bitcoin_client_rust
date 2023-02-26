use std::convert::TryFrom;

use serde::{Serialize, Deserialize};
use ring::digest::{digest, SHA256, Digest};

// 20-byte address
#[derive(Eq, PartialEq, Serialize, Deserialize, Clone, Hash, Default, Copy)]
pub struct Address([u8; 20]);

impl std::convert::From<&[u8; 20]> for Address {
    fn from(input: &[u8; 20]) -> Address {
        let mut buffer: [u8; 20] = [0; 20];
        buffer[..].copy_from_slice(input);
        Address(buffer)
    }
}

impl std::convert::From<[u8; 20]> for Address {
    fn from(input: [u8; 20]) -> Address {
        Address(input)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let start = if let Some(precision) = f.precision() {
            if precision >= 40 {
                0
            } else {
                20 - precision / 2
            }
        } else {
            0
        };
        for byte_idx in start..20 {
            write!(f, "{:>02x}", &self.0[byte_idx])?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:>02x}{:>02x}..{:>02x}{:>02x}",
            &self.0[0], &self.0[1], &self.0[18], &self.0[19]
        )
    }
}

impl Address {
    /* Takes a key, hashes it, and uses the last 20 bytes as a Bitcoin address*/
    pub fn from_public_key_bytes(bytes: &[u8]) -> Address {
        //hash the key
        let hashed_key_digest: Digest = digest(&SHA256, bytes);
        let hashed_key: &[u8] = hashed_key_digest.as_ref();
        //return an Address made with the last 20 bytes of the hashed key
        let last_20_bytes: [u8; 20] = <[u8; 20]>::try_from(&hashed_key[hashed_key.len()-20..hashed_key.len()]).unwrap();
        return Address(last_20_bytes);
    }
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use super::Address;

    #[test]
    fn from_a_test_key() {
        let test_key = hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d");
        let addr: Address = Address::from_public_key_bytes(&test_key);
        let correct_addr: Address = hex!("1851a0eae0060a132cf0f64a0ffaea248de6cba0").into();
        assert_eq!(addr, correct_addr);
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // take the last 20 bytes, we get "1851a0eae0060a132cf0f64a0ffaea248de6cba0"
    }
    #[test]
    fn from_a_test_key_2() {
        let test_key = hex!("1234");
        let addr: Address = Address::from_public_key_bytes(&test_key);
        let correct_addr: Address = hex!("e39accfbc0ae208096437401b7ceab63cca0622f").into();
        assert_eq!(addr, correct_addr);
        // "3a103a4e5729ad68c02a678ae39accfbc0ae208096437401b7ceab63cca0622f" is the hash of
        // "1234"
        // take the last 20 bytes, we get "e39accfbc0ae208096437401b7ceab63cca0622f"
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST