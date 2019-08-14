#[macro_use]
extern crate log;

pub use ws::Result;
use ws::connect;
use serde_json::json;
pub use primitives::H256 as Hash;
use crossbeam;
use crossbeam::channel::unbounded;
use parity_codec::Encode;
use runtime_primitives::traits::Extrinsic;
use std::sync::mpsc::Sender as StdThreadOut;
use std::sync::mpsc::channel;
use std::thread;

pub mod utils;
pub mod handler;
pub use utils::*;
use handler::*;

const WS_URL_LOCAL: &str = "ws://127.0.0.1:9944";
const REQUEST_SUBMIT: u32 = 2;

/// Endpoint to connect substrate nodes
pub enum Url {
    /// Connecting localhost, 9944 port.
    Local,
    /// Connecting customized endpoint.
    Custom(String),
}

/// Define url for json-rpc over websocket
#[derive(Clone, Debug)]
pub struct Api(String);

impl Api {
    pub fn init(url: Url) -> Self {
        match url {
            Url::Local => Api(WS_URL_LOCAL.to_owned()),
            Url::Custom(url) => Api(url.to_owned()),
        }
    }

    // -----------------------------------
    //  High-level API
    // -----------------------------------

    /// Get the specified account id's nonce which is stored in `System` module.
    pub fn get_nonce<E>(&self, account_id: &E) -> Result<u64>
    where
        E: Encode,
    {
        let index_str = self.get_storage("System", "AccountNonce", Some(account_id.encode()))?;
        Ok(hexstr_to_u64(index_str))
    }

    /// Submit an extrinsic to substrate nodes
    pub fn submit_extrinsic<E>(&self, uxt: &E) -> Result<Hash>
    where
        E: Extrinsic + Encode,
    {
        let mut uxt_hex = hex::encode(uxt.encode());
        uxt_hex.insert_str(0, "0x");
        self.submit_raw_extrinsic(uxt_hex)
    }


    // -----------------------------------
    //  Low-level API
    // -----------------------------------

    pub fn get_storage(&self, module: &str, storage_key: &str, params: Option<Vec<u8>>) -> Result<String> {
        let key_hash = storage_key_hash(module, storage_key, params);
        let req = json!({
            "method": "state_getStorage",
            "params": [key_hash],
            "jsonrpc": "2.0",
            "id": "1",
        });

        get_response(&self.0[..], req.to_string())
    }

    pub fn get_latest_blockhash(&self) -> Result<String> {
        let req = json!({
            "method": "chain_getBlockHash",
            "params": [],
            "jsonrpc": "2.0",
            "id": "1",
        });

        get_response(&self.0[..], req.to_string())
    }

    pub fn get_genesis_blockhash(&self) -> Result<Hash> {
        let req = json!({
            "method": "chain_getBlockHash",
            "params": [0],
            "jsonrpc": "2.0",
            "id": "1",
        });

        let res = get_response(&self.0[..], req.to_string()).unwrap();
        Ok(hexstr_to_hash(res))
    }

    pub fn get_latest_height(&self) -> Result<String> {
        let req = json!({
            "method": "chain_getHeader",
            "params": [],
            "jsonrpc": "2.0",
            "id": "1",
        });

        get_height_response(&self.0[..], req.to_string())
    }

    pub fn get_runtime_version(&self) -> Result<String> {
        let req = json!({
            "method": "state_getRuntimeVersion",
            "params": [],
            "jsonrpc": "2.0",
            "id": "1",
        });

        get_response(&self.0[..], req.to_string())
    }

    pub fn submit_raw_extrinsic(&self, params: String) -> Result<Hash> {
        let req = json!({
            "method": "author_submitAndWatchExtrinsic",
            "params": [params],
            "jsonrpc": "2.0",
            "id": REQUEST_SUBMIT.to_string(), // TODO
        });

        submit(&self.0[..], req.to_string())
    }

    pub fn subscribe_events(&self, sender: StdThreadOut<String>) {
        let key_hash = storage_key_hash("System", "Events", None);
        let req = json!({
            "method": "state_subscribeStorage",
            "params": [[key_hash]],
            "jsonrpc": "2.0",
            "id": "1",
        }).to_string();

        let (tx, rx) = channel();
        let url = self.0.clone();

        let _client = thread::Builder::new()
            .name("client".to_string())
            .spawn(move || {
                connect(url, |output| {
                    SubscriptionHandler {
                        output,
                        request: req.clone(),
                        result: tx.clone(),
                    }
                }).expect("must connect")
            }).expect("must run");

        loop {
            let res: String = rx.recv().expect("must not be empty");
            sender.send(res.clone()).unwrap();
        }
    }
}

fn get_response(url: &str, req: String) -> Result<String> {
    let (tx, rx) = unbounded();

    crossbeam::scope(|scope| {
        scope.spawn(move |_| {
            connect(url.to_owned(), |output| {
                Getter {
                    output,
                    request: req.clone(),
                    result: tx.clone(),
                }
            }).expect("must connect")
        });
    }).expect("must run");

    Ok(rx.recv().expect("must not be empty"))
}

fn get_height_response(url: &str, req: String) -> Result<String> {
    let (tx, rx) = unbounded();

    crossbeam::scope(|scope| {
        scope.spawn(move |_| {
            connect(url.to_owned(), |output| {
                HeightGetter {
                    output,
                    request: req.clone(),
                    result: tx.clone(),
                }
            }).expect("must connect")
        });
    }).expect("must run");

    Ok(rx.recv().expect("must not be empty"))
}

// TODO: Getting abstract
fn submit(url: &str, req: String) -> Result<Hash> {
    let (tx, rx) = unbounded();

    crossbeam::scope(|scope| {
        scope.spawn(move |_| {
            connect(url.to_owned(), |output| {
                Submitter {
                    output,
                    request: req.clone(),
                    result: tx.clone(),
                }
            }).expect("must connect")
        });
    }).expect("must run");

    Ok(rx.recv().expect("must not be empty"))
}

// fn subscribe(url: String, req: String, sender: StdThreadOut<String>) -> Result<()> {
//     let (tx, rx) = channel();

//     let _client = thread::Builder::new()
//         .name("client".to_string())
//         .spawn(move || {
//             connect(url, |output| {
//                 SubscriptionHandler {
//                     output,
//                     request: req.clone(),
//                     result: sender.clone(),
//                 }
//             }).expect("must connect")
//         }).expect("must run");

//     // crossbeam::scope(|scope| {
//     //     scope.spawn(move |_| {
//     //         connect(url.to_owned(), |output| {
//     //             SubscriptionHandler {
//     //                 output,
//     //                 request: req.clone(),
//     //                 result: sender.clone(),
//     //             }
//     //         }).expect("must connect")
//     //     });
//     // }).expect("must run");
// // println!("Subscribe to events!!!!!!");
//     loop {
//         let res: String = rx.recv().expect("must not be empty");
//         sender.send(res.clone()).unwrap();
//     }
// }

#[cfg(test)]
mod tests{
    use super::*;
    use hex_literal::{hex, hex_impl};
    use parity_codec::{Encode, Compact, Decode};
    use primitives::blake2_256;
    use runtime::{UncheckedExtrinsic, Call, EncryptedBalancesCall, Event};
    use runtime_primitives::generic::Era;
    use zprimitives::{Proof, Ciphertext, PkdAddress, SigVerificationKey, RedjubjubSignature};
    use zjubjub::{curve::{
        fs::Fs as zFs,
        JubjubBls12 as zJubjubBls12,
        FixedGenerators as zFixedGenerators}, redjubjub::{PrivateKey, PublicKey}};
    use rand::{XorShiftRng, SeedableRng};
    use zpairing::{bls12_381::Bls12 as zBls12, PrimeField, PrimeFieldRepr};

    #[test]
    fn test_get_storage() {
        let api = Api::init(Url::Local);
        let res_str = api.get_storage("Balances", "ExistentialDeposit", None).unwrap();
        let res = hexstr_to_u64(res_str);
        println!("TransactionBaseFee is {}", res);

        let pkd_addr_alice: [u8; 32] = hex!("fd0c0c0183770c99559bf64df4fe23f77ced9b8b4d02826a282bcd125117dcc2");
        let alice_address = PkdAddress::from_slice(&pkd_addr_alice);

        let res_str = api.get_storage("EncryptedBalances", "LastRollover", Some(alice_address.encode())).unwrap();
        let res = hexstr_to_u64(res_str);
        println!("LastRollover is {}", res);

        let mut res = api.get_storage("EncryptedBalances", "EncryptedBalance", Some(alice_address.encode())).unwrap();
        println!("Encrypted balance: {:?}", res);

        for _ in 0..4 {
            res.remove(2);
        }

        println!("Encrypted balance: {:?}", res);
    }

    #[test]
    fn test_get_runtime_version() {
        let api = Api::init(Url::Local);
        let res_str = api.get_runtime_version().unwrap();
        println!("Runtime version is {}", res_str);
    }

    #[test]
    fn  test_get_latest_header() {
        let api = Api::init(Url::Local);
        let res_str = api.get_latest_height().unwrap();
        let res = hexstr_to_u64(res_str);

        println!("res:{}", res);
    }

    #[test]
    fn test_submit_extrinsic() {
        let api = Api::init(Url::Local);

        let proof: [u8; 192] = hex!("a127994f62fefa882271cbe9fd1fffd16bcf3ebb3cd219c04be4333118f33115e4c70e8d199e43e956b8761e1e69bff48ff156d14e7d083a09e341da114b05a5c2eff9bd6aa9881c7ca282fbb554245d2e65360fa72f1de6b538b79a672cdf86072eeb911b1dadbfef2091629cf9ee76cf80ff7ec085258b102caa62f5a2a00b48dce27c91d59c2cdfa23b456c0f616ea1e9061b5e91ec080f1c3c66cf2e13ecca7b7e1530addd2977a123d6ebbea11e9f8c3b1989fc830a309254e663315dcb");
        let pkd_addr_alice: [u8; 32] = hex!("fd0c0c0183770c99559bf64df4fe23f77ced9b8b4d02826a282bcd125117dcc2");
        let pkd_addr_bob: [u8; 32] = hex!("45e66da531088b55dcb3b273ca825454d79d2d1d5c4fa2ba4a12c1fa1ccd6389");
        let enc10_by_alice: [u8; 64] = hex!("29f38e21e264fb8fa61edc76f79ca2889228d36e40b63f3697102010404ae1d0b8b965029e45bd78aabe14c66458dd03f138aa8b58490974f23aabb53d9bce99");
        let enc10_by_bob: [u8; 64] = hex!("4c6bda3db6977c29a115fbc5aba03b9c37b767c09ffe6c622fcec42bbb732fc7b8b965029e45bd78aabe14c66458dd03f138aa8b58490974f23aabb53d9bce99");
        let enc1_by_alice: [u8; 64] = hex!("ed19f1820c3f09da976f727e8531aa83a483d262e4abb1e9e67a1eba843b4034b8b965029e45bd78aabe14c66458dd03f138aa8b58490974f23aabb53d9bce99");
        let rvk: [u8; 32] = hex!("f539db3c0075f6394ff8698c95ca47921669c77bb2b23b366f42a39b05a88c96");
        let rsk_bytes: [u8; 32] = hex!("a36bb97dbe99b6c9e1f58b44130797d088d5439d97748229772449b29ec15909");

        let sig_vk = SigVerificationKey::from_slice(&rvk[..]);

        let calls = Call::EncryptedBalances(EncryptedBalancesCall::confidential_transfer(
            Proof::from_slice(&proof[..]),
            PkdAddress::from_slice(&pkd_addr_alice[..]),
            PkdAddress::from_slice(&pkd_addr_bob[..]),
            Ciphertext::from_slice(&enc10_by_alice[..]),
            Ciphertext::from_slice(&enc10_by_bob[..]),
            Ciphertext::from_slice(&enc1_by_alice[..]),
        ));

        let height_str = api.get_latest_height().unwrap();
        let height = hexstr_to_u64(height_str);
        println!("height: {}", height);
        let era = Era::immortal(); // TODO
        // let era = Era::mortal(256, height);

        let index = api.get_nonce(&sig_vk).unwrap();
        println!("index: {}", index);

        let checkpoint = api.get_genesis_blockhash().unwrap();

        println!("block_hash: {:?}", checkpoint.clone());

        // let raw_payload = (Compact(index), calls, era, checkpoint);
        let raw_payload = (Compact(index), calls.clone(), era, checkpoint);

        println!("calls: {:?}", hex::encode(calls.encode()));

        let mut rsk_repr = zFs::default().into_repr();
        rsk_repr.read_le(&mut &rsk_bytes[..]).unwrap();
        let rsk = zFs::from_repr(rsk_repr).unwrap();
        let sk = PrivateKey::<zBls12>(rsk);

        let params = &zJubjubBls12::new();
        let rng = &mut XorShiftRng::from_seed([0xbc4f6d44, 0xd62f276c, 0xb963afd0, 0x5455863d]);
        let p_g = zFixedGenerators::Diversifier;

        let vk = PublicKey::<zBls12>::read(&mut &rvk[..], params).unwrap();

        let sig = raw_payload.using_encoded(|payload| {
            if payload.len() > 256 {
                println!("payload: {:?}", hex::encode(payload));
                let msg = blake2_256(payload);
                let sig = sk.sign(&msg[..] ,rng, p_g, params);

                // verify signature
                assert!(vk.verify(&msg, &sig, p_g, params));
                println!("msg: {:?}", hex::encode(msg.encode()));

                sig
            } else {
                sk.sign(payload ,rng, p_g, params)
            }
        });

        let sig = RedjubjubSignature::from_signature(&sig);
        println!("sig: {:?}", sig);
        let uxt = UncheckedExtrinsic::new_signed(index, raw_payload.1, sig_vk.into(), sig, era);

        println!("index:{:?}", index);
        println!("era: {:?}", &era.encode()[..]);
        println!("Start sending tx....");
        let _tx_hash = api.submit_extrinsic(&uxt).unwrap();
    }

    #[test]
    fn test_subscribe_event() {
        let api = Api::init(Url::Local);
        let (tx, rx) = channel();

        println!("Subscribe to events");

        let _event_sbscriber = thread::Builder::new()
            .name("eventsubscriber".to_string())
            .spawn(move || {
                api.subscribe_events(tx.clone());
            }).unwrap();

        loop {
            let event_str = rx.recv().unwrap();
            let res_vec = hexstr_to_vec(event_str);
            let mut er_enc = res_vec.as_slice();
            let events = Vec::<system::EventRecord::<Event>>::decode(&mut er_enc);
            match events {
                Some(events) => {
                    for event in &events {
                        match &event.event {
                            Event::encrypted_balances(enc_be) => {
                                println!("encrypted balance event: {:?}", enc_be);
                                match &enc_be {
                                    encrypted_balances::RawEvent::ConfidentialTransfer(
                                        zkproof,
                                        address_sender,
                                        address_recipient,
                                        amount_sender,
                                        amount_recipient,
                                        fee_sender,
                                        enc_balances,
                                        sig_vk
                                    ) => {
                                        println!("zk proof: {:?}", zkproof);
                                        println!("address_sender: {:?}", address_sender);
                                        println!("address_recipient: {:?}", address_recipient);
                                        println!("amount_sender: {:?}", amount_sender);
                                        println!("amount_recipient: {:?}", amount_recipient);
                                        println!("fee_sender: {:?}", fee_sender);
                                        println!("enc_balances: {:?}", enc_balances);
                                        println!("zk proof: {:?}", sig_vk);
                                    },
                                    _ => {
                                        println!("ignoring unsupported encrypted_balances event");
                                    }
                                }
                            }
                            _ => {
                                println!("ignoring unsupported module event: {:?}", event.event)
                            }
                        }
                    }
                },
                None => error!("couldn't decode event record list")
            }
        }
    }
}
