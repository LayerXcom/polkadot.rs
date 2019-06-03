use crate::Hash;
use hex;
use primitives::twox_128;
use byteorder::{LittleEndian, ByteOrder};

pub fn hexstr_to_hash(hexstr: String) -> Hash {
    let vec = hexstr_to_vec(hexstr);
    let mut gh: [u8; 32] = Default::default();
    gh.copy_from_slice(&vec);

    Hash::from(gh)
}

pub fn hexstr_to_u64(hexstr: String) -> u64 {
    let mut vec = hexstr_to_vec(hexstr);

    if vec.len() % 8 != 0 {
        for _ in 0..(8 - vec.len() % 8) {
            // vec.insert(0, 0);
            vec.push(0);
        }
    }
    LittleEndian::read_u64(&vec[..])
}

pub fn hexstr_to_vec(hexstr: String) -> Vec<u8> {
    let mut hexstr = hexstr.clone();
    if hexstr.starts_with("0x") {
        hexstr.remove(0);
        hexstr.remove(0);
    }
    else {
        panic!("converting non-prefixed hex string")
    }

    if hexstr.len() % 2 == 0 {
        hex::decode(&hexstr).unwrap()
    } else {
        let zero: String = "0".to_string();
        hex::decode(&format!("{}{}", zero, hexstr)).unwrap()
    }
}

pub fn storage_key_hash(module: &str, storage_key: &str, params: Option<Vec<u8>>) -> String {
        let mut key = module.as_bytes().to_vec();
        key.append(&mut vec!(' ' as u8));
        key.append(&mut storage_key.as_bytes().to_vec());

        let mut key_hash;
        match params {
            Some(mut p) => {
                key.append(&mut p);
                key_hash = hex::encode(twox_128(&key));
                },
            None => {
                key_hash = hex::encode(twox_128(&key));
                },
        }

        key_hash.insert_str(0, "0x");
        key_hash
}
