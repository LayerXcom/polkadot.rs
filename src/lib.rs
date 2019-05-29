// use jsonrpc_core_client::transports::ws::connect;
use ws::{connect, Result, Handler, Sender, Message, Handshake};
use serde_json::json;
use node_primitives::Hash;
use std::sync::mpsc::{channel, Sender as ThreadOut};
use std::thread;

pub mod utils;
use utils::*;

const WS_URL_LOCAL: &str = "ws://127.0.0.1:9944";

pub enum Url {
    Local,
    Custom(&'static str),
}

pub struct Api {
    url: String,
    genesis_hash: Hash,
}

impl Api {
    pub fn connect(url: Url) -> Self {
        let json_req = json!({
            "method": "chain_getBlockHash",
            "params": [0],
            "jsonrpc": "2.0",
            "id": "1",
        });

        match url {
            Url::Local => {
                let genesis_hash_str = get_request(WS_URL_LOCAL, json_req.to_string()).unwrap();

                Api {
                    url: WS_URL_LOCAL.to_owned(),
                    genesis_hash: hexstr_to_hash(genesis_hash_str)
                }
            },
            Url::Custom(url) => {
                let genesis_hash_str = get_request(url, json_req.to_string()).unwrap();

                Api {
                    url: url.to_owned(),
                    genesis_hash: hexstr_to_hash(genesis_hash_str)
                }
            }
        }
    }
}

pub fn get_request(url: &'static str, req: String) -> Result<String> {
    let (tx, rx) = channel();
    let client = thread::Builder::new()
        .name("client".to_owned())
        .spawn(move || {
            connect(url.to_owned(), |out| {
                Getter {
                    out,
                    request: req.clone(),
                    result: tx.clone(),
                }
            }).unwrap()
        }).unwrap();

    Ok(rx.recv().unwrap())
}

struct Getter {
    out: Sender,
    request: String,
    result: ThreadOut<String>,
}

impl Handler for Getter {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        unimplemented!();
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        unimplemented!();
    }
}

// #[cfg(test)]
// mod tests{
//     use super::*;
//     use futures::prelude::*;
//     use jsonrpc_core_client::{RpcChannel, RpcError};
//     use jsonrpc_core::{IoHandler, Result};
//     use failure::Error;
//     use substrate_rpc::system::{System, SystemApi};
//     use jsonrpc_derive::rpc;

//     impl From<RpcChannel> for System {
//         fn from(channel: RpcChannel) -> Self {

//         }
//     }

//     #[test]
//     fn test_try() {
//         let mut io = IoHandler::new();
//         io.extend_with(System.to_delegate());

//         let client = connect::<_>(WS_URL_LOCAL).unwrap();
//         client.wait().unwrap().system_name().map(|res| println!("res: {:?}", res));

//     }
// }
