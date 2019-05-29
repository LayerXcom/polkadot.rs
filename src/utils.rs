use node_primitives::Hash;
use primitive_types::U256;
use hex;
use primitives::{twox_128, blake2_256};

pub fn hexstr_to_hash(hexstr: String) -> Hash {
    let vec = hexstr_to_vec(hexstr);
    let mut gh: [u8; 32] = Default::default();

    gh.copy_from_slice(&vec);
    Hash::from(gh)
}

pub fn hexstr_to_u256(hexstr: String) -> U256 {
    let vec = hexstr_to_vec(hexstr);
    U256::from_little_endian(&mut &vec[..])
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
    hex::decode(&hexstr).unwrap()
}

pub fn storage_key_hash(module: &str, storage_key_name: &str, param: Option<Vec<u8>>) -> String {
        let mut key = module.as_bytes().to_vec();
        key.append(&mut vec!(' ' as u8));
        key.append(&mut storage_key_name.as_bytes().to_vec());
        let mut keyhash;
        match param {
            Some(par) => {
                key.append(&mut par.clone());
                keyhash = hex::encode(blake2_256(&key));
                },
            _ => {
                keyhash = hex::encode(twox_128(&key));
                },
        }
        keyhash.insert_str(0, "0x");
        keyhash
}
