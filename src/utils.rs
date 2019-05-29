use node_primitives::Hash;
use primitive_types::U256;
use hex;

pub fn hexstr_to_hash(hexstr: String) -> Hash {
    let _unhex = hexstr_to_vec(hexstr);
    let mut gh: [u8; 32] = Default::default();
    gh.copy_from_slice(&_unhex);
    Hash::from(gh)
}

pub fn hexstr_to_u256(hexstr: String) -> U256 {
    let _unhex = hexstr_to_vec(hexstr);
    U256::from_little_endian(&mut &_unhex[..])
}

pub fn hexstr_to_vec(hexstr: String) -> Vec<u8> {
    let mut _hexstr = hexstr.clone();
    if _hexstr.starts_with("0x") {
        _hexstr.remove(0);
        _hexstr.remove(0);
    }
    else {
        panic!("converting non-prefixed hex string")
    }
    hex::decode(&_hexstr).unwrap()
}
