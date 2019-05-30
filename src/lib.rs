// use jsonrpc_core_client::transports::ws::connect;
use ws::{connect, Result, Handler, Sender, Message, Handshake, CloseCode};
use serde_json::json;
use node_primitives::Hash;
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
    use zprimitives::pkd_address::PkdAddress;
    use parity_codec::{Encode, Compact};
    use primitives::{/*ed25519, sr25519, Pair,*/ blake2_256};
    // use runtime::{UncheckedExtrinsic, Call, ConfTransferCall};
    // use runtime_primitives::generic::Era;
    use primitive_types::U256;

    // pub fn transfer(from: &str, to: &str, amount: U256, genesis_hash: Hash) -> UncheckedExtrinsic {
	// 	let signer = Sr25519::pair_from_suri(from, Some(""));

	// 	let to = sr25519::Public::from_string(to).ok().or_else(||
	// 		sr25519::Pair::from_string(to, Some("")).ok().map(|p| p.public())
	// 	).expect("Invalid 'to' URI; expecting either a secret URI or a public URI.");
	// 	let amount = Balance::from(amount.low_u128());
	// 	// let index = Index::from(index.low_u64());
    //     let index = 0 as u64;

	// 	let function = Call::Balances(BalancesCall::transfer(to.into(), amount));

	// 	let era = Era::immortal();

	// 	debug!("using genesis hash: {:?}", genesis_hash);
	// 	let raw_payload = (Compact(index), function, era, genesis_hash);
	// 	let signature = raw_payload.using_encoded(|payload| if payload.len() > 256 {
	// 		signer.sign(&blake2_256(payload)[..])
	// 	} else {
	// 		signer.sign(payload)
	// 	});
	// 	UncheckedExtrinsic::new_signed(
	// 		index,
	// 		raw_payload.1,
	// 		signer.public().into(),
	// 		signature.into(),
	// 		era,
	// 	)
	// }

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

    }
}
