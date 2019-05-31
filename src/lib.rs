// use jsonrpc_core_client::transports::ws::connect;
use ws::{connect, Result, Handler, Sender, Message, Handshake, CloseCode};
use serde_json::json;
use runtime::Hash; // TODO
use ws::{ErrorKind, Error};
use crossbeam;
use crossbeam::channel::{unbounded, Sender as ThreadOut};

pub mod utils;
use utils::*;

const WS_URL_LOCAL: &str = "ws://127.0.0.1:9944";
const REQUEST_SUBMIT: u32 = 3;

pub enum Url {
    Local,
    Custom(&'static str),
}

pub struct Api {
    url: String
}

impl Api {
    pub fn init(url: Url) -> Result<Self> {
        match url {
            Url::Local => {
                Ok(Api {
                    url: WS_URL_LOCAL.to_owned()
                })
            },
            Url::Custom(url) => {
                Ok(Api {
                    url: url.to_owned()
                })
            }
        }
    }

    pub fn get_storage(&self, module: &str, storage_key: &str, params: Option<Vec<u8>>) -> Result<String> {
        let key_hash = storage_key_hash(module, storage_key, params);
        let req = json!({
            "method": "state_getStorage",
            "params": [key_hash],
            "jsonrpc": "2.0",
            "id": "1",
        });

        get_response(&self.url[..], req.to_string())
    }

    pub fn get_latest_blockhash(&self) -> Result<String> {
        let req = json!({
            "method": "chain_getBlockHash",
            "params": [],
            "jsonrpc": "2.0",
            "id": "1",
        });

        get_response(&self.url[..], req.to_string())
    }

    pub fn submit_extrinsic(&self, params: String) -> Result<Hash> {
        let req = json!({
            "method": "author_submitAndWatchExtrinsic",
            "params": [params],
            "jsonrpc": "2.0",
            "id": REQUEST_SUBMIT.to_string(), // TODO
        });

        submit(&self.url[..], req.to_string())
    }
}

struct Getter {
    /// A representation of the output of the WebSocket connection.
    output: Sender,
    /// The json request data which is formatted string type.
    request: String,
    /// The sending side of a channel.
    result: ThreadOut<String>,
}

impl Handler for Getter {
    /// Called when the WebSocket handshake is successful and the connection is open for sending
    /// and receiving messages.
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        self.output.send(self.request.clone())
            .map_err(|_| Error::new(ErrorKind::Internal, "must connect"))?;

        Ok(())
    }

    /// Called on incoming messages.
    fn on_message(&mut self, msg: Message) -> Result<()> {
        let txt = msg.as_text()?;
        let value: serde_json::Value = serde_json::from_str(txt)
            .map_err(|_| Error::new(ErrorKind::Internal, "Request deserialization is infallible; qed"))?;

        let hex_str = match value["result"].as_str() {
            Some(res) => res.to_string(),
            None => "0x00".to_string(),
            // None => return Err(Error::new(ErrorKind::Internal, "No result in the storage key of the module")),
        };

        self.result.send(hex_str)
            .map_err(|_| Error::new(ErrorKind::Internal, "must run"))?;
        self.output.close(CloseCode::Normal)?;
        Ok(())
    }
}

struct Submitter {
    output: Sender,
    request: String,
    result: ThreadOut<Hash>,
}

impl Handler for Submitter {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        self.output.send(self.request.clone())
            .map_err(|_| Error::new(ErrorKind::Internal, "must connect"))?;

        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let txt = msg.as_text()?;
        let value: serde_json::Value = serde_json::from_str(txt)
            .map_err(|_| Error::new(ErrorKind::Internal, "Request deserialization is infallible; qed"))?;

        match value["id"].as_str() {
            Some(id) => {
                match id.parse::<u32>() {
                    Ok(REQUEST_SUBMIT) => {
                        match value.get("error") {
                            Some(err) => {},
                            None => {},
                        }
                    },
                    Ok(_) => {

                    },
                    Err(_) => {

                    }
                }
            },
            None => {
                match value["method"].as_str() {
                    Some("author_extrinsicUpdate") => {
                        match value["params"]["result"].as_str() {
                            Some(res) => {},
                            None => {
                                self.result.send(hexstr_to_hash(value["params"]["result"]["finalized"].as_str().unwrap().to_string())).unwrap();
                                self.output.close(CloseCode::Normal).unwrap();
                            },
                        }
                    },
                    Some(_) => {},
                    None => {},
                }
            }
        };

        Ok(())
    }
}

pub fn get_response(url: &str, req: String) -> Result<String> {
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

// TODO: Getting abstract
pub fn submit(url: &str, req: String) -> Result<Hash> {
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

#[cfg(test)]
mod tests{
    use super::*;
    use hex_literal::{hex, hex_impl};
    use parity_codec::{Encode, Compact};
    use primitives::blake2_256;
    use runtime::{UncheckedExtrinsic, Call, ConfTransferCall};
    use runtime_primitives::generic::Era;
    use primitive_types::U256;
    use zprimitives::{Proof, Ciphertext, PkdAddress, SigVerificationKey};

    #[test]
    fn test_get_storage() {
        let api = Api::init(Url::Local).unwrap();
        let res_str = api.get_storage("Balances", "ExistentialDeposit", None).unwrap();
        // let res = api.get_storage("ConfTransfer", "VerifyingKey", None).unwrap();
        let res = hexstr_to_u256(res_str);
        println!("TransactionBaseFee is {}", res);

        let pkd_addr_alice: [u8; 32] = hex!("fd0c0c0183770c99559bf64df4fe23f77ced9b8b4d02826a282bcd125117dcc2");
        let alice_address = PkdAddress::from_slice(&pkd_addr_alice);
        let res = api.get_storage("ConfTransfer", "EncryptedBalance", Some(alice_address.encode())).unwrap();
        println!("Encrypted balance: {:?}", res);
    }

    #[test]
    fn test_submit_extrinsic() {
        let api = Api::init(Url::Local).unwrap();

        let proof: [u8; 192] = hex!("8ff35054c963afa7e0cbfd42e4517a4ab10a31798134f8d67d95800d788c804dd59dbe551d9f11426c77b567b803b5428aad134e1946a392153c1ab597d763faaa108ac7a7736759811b34252500db10cc40ba70fbbfe2dd71e2d1ee57b6f5791426df5cf6e36e6ec0a92fab2e76403a84c8bccb724429698d794be760f88d488cbbf031afcebed75a996a0e151a5ade889a8ac6a528481444b53949292177136c887afa22f484b7e509bbde20187e7ed3335e53453f010639cab8af1f0b927b");
        let accountid_sender: [u8; 32] = hex!("fd0c0c0183770c99559bf64df4fe23f77ced9b8b4d02826a282bcd125117dcc2");
        let accountid_recipient: [u8; 32] = hex!("45e66da531088b55dcb3b273ca825454d79d2d1d5c4fa2ba4a12c1fa1ccd6389");
        let enc10_by_sender: [u8; 64] = hex!("5e4d370d5ca213b8da2c14b192cd5ce9176faaf8a0e94f64bb649ccbe7cad827df97523bf003405c38dd66d3a793169618b02e0f13d2a8d669657ffb81a01c33");
        let enc10_by_recipient: [u8; 64] = hex!("690faa236b77eeceb0940429c8abed2721a83cf4dcd32b5f8bc0e886a39d8ae9df97523bf003405c38dd66d3a793169618b02e0f13d2a8d669657ffb81a01c33");
        let rvk: [u8; 32] = hex!("f539db3c0075f6394ff8698c95ca47921669c77bb2b23b366f42a39b05a88c96");

        let calls = Call::ConfTransfer(ConfTransferCall::confidential_transfer(
            Proof::from_slice(&proof[..]),
            PkdAddress::from_slice(&accountid_sender[..]),
            PkdAddress::from_slice(&accountid_recipient[..]),
            Ciphertext::from_slice(&enc10_by_sender[..]),
            Ciphertext::from_slice(&enc10_by_recipient[..]),
            SigVerificationKey::from_slice(&rvk[..])
        ));

        // println!("fun: {:?}", calls);

        let era = Era::immortal(); // TODO
        let index = 0 as u64;

        let block_hash = api.get_latest_blockhash().unwrap();

        println!("block_hash: {:?}", block_hash);

        let raw_payload = (Compact(index), calls, era, block_hash);

    }
}
