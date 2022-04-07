use serde::Serialize;
use crate::blockchain::Blockchain;
use crate::miner::Handle as MinerHandle;
use crate::transaction_generator::Handle as TxGeneratorHandle;
use crate::network::server::Handle as NetworkServerHandle;
use crate::network::message::Message;
use crate::types::address::Address;
use crate::types::block::BlockState;
use crate::types::hash::{H256, Hashable};

use log::{info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::Header;
use tiny_http::Response;
use tiny_http::Server as HTTPServer;
use url::Url;

pub struct Server {
    handle: HTTPServer,
    miner: MinerHandle,
    tx_generator: TxGeneratorHandle,
    network: NetworkServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    block_state: Arc<Mutex<BlockState>>
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

macro_rules! respond_result {
    ( $req:expr, $success:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let payload = ApiResponse {
            success: $success,
            message: $message.to_string(),
        };
        let resp = Response::from_string(serde_json::to_string_pretty(&payload).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}
macro_rules! respond_json {
    ( $req:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let resp = Response::from_string(serde_json::to_string(&$message).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}

impl Server {
    pub fn start(
        addr: std::net::SocketAddr,
        miner: &MinerHandle,
        tx_generator: &TxGeneratorHandle,
        network: &NetworkServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>,
        block_state: &Arc<Mutex<BlockState>>
    ) {
        let handle = HTTPServer::http(&addr).unwrap();
        let server = Self {
            handle,
            miner: miner.clone(),
            tx_generator: tx_generator.clone(),
            network: network.clone(),
            blockchain: Arc::clone(blockchain),
            block_state: Arc::clone(block_state)
        };
        thread::spawn(move || {
            for req in server.handle.incoming_requests() {
                let miner = server.miner.clone();
                let tx_generator = server.tx_generator.clone();
                let network = server.network.clone();
                let blockchain = Arc::clone(&server.blockchain);
                let block_state_map = Arc::clone(&server.block_state);
                thread::spawn(move || {
                    // a valid url requires a base
                    let base_url = Url::parse(&format!("http://{}/", &addr)).unwrap();
                    let url = match base_url.join(req.url()) {
                        Ok(u) => u,
                        Err(e) => {
                            respond_result!(req, false, format!("error parsing url: {}", e));
                            return;
                        }
                    };
                    match url.path() {
                        "/miner/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let lambda = match params.get("lambda") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing lambda");
                                    return;
                                }
                            };
                            let lambda = match lambda.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing lambda: {}", e)
                                    );
                                    return;
                                }
                            };
                            miner.start(lambda);
                            respond_result!(req, true, "ok");
                        }
                        "/tx-generator/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let theta = match params.get("theta") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing theta");
                                    return;
                                }
                            };
                            let theta = match theta.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing theta: {}", e)
                                    );
                                    return;
                                }
                            };
                            tx_generator.start(5000*theta);
                            respond_result!(req, true, "ok");
                        }
                        "/network/ping" => {
                            network.broadcast(Message::Ping(String::from("Test ping")));
                            respond_result!(req, true, "ok");
                        }
                        "/blockchain/longest-chain" => {
                            let v = blockchain.lock().unwrap().all_blocks_in_longest_chain().clone();
                            let v_string: Vec<String> = v.into_iter().map(|h|h.to_string()).collect();
                            respond_json!(req, v_string);
                        }
                        "/blockchain/longest-chain-tx" => {
                            let blocks = blockchain.lock().unwrap().all_blocks_in_longest_chain().clone();
                            let block_map = blockchain.lock().unwrap().block_map.clone();
                            let mut txs = Vec::<Vec::<H256>>::new();
                            for block_hash in blocks.clone() {
                                let mut txs2 = Vec::<H256>::new();
                                let (block, _) = block_map.get(&block_hash).unwrap();
                                for transaction in block.get_content().data.clone() {
                                    txs2.push(transaction.hash());
                                }
                                txs.push(txs2);
                            }
                            let mut txs_string: Vec<Vec<String>> = Vec::<Vec<String>>::new();
                            for vec in txs {
                                let vecs: Vec<String> = vec.into_iter().map(|h|h.to_string()).collect();
                                txs_string.push(vecs);
                            }
                            respond_json!(req, txs_string);
                        }
                        "/blockchain/longest-chain-tx-count" => {
                            respond_result!(req, false, "unimplemented!");
                        }
                        "/blockchain/state" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let block = match params.get("block") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing block");
                                    return;
                                }
                            };
                            let block = match block.parse::<usize>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing block: {}", e)
                                    );
                                    return;
                                }
                            };
                            //so that ordering is consistent across API calls
                            let accounts: [Address; 3] = [
                                Address::from_public_key_bytes(&[59, 106, 39, 188, 206, 182, 164, 45, 98, 163, 168, 208, 42, 111, 13, 115, 101, 50, 21, 119, 29, 226, 67, 166, 58, 192, 72, 161, 139, 89, 218, 41]),
                                Address::from_public_key_bytes(&[138, 136, 227, 221, 116, 9, 241, 149, 253, 82, 219, 45, 60, 186, 93, 114, 202, 103, 9, 191, 29, 148, 18, 27, 243, 116, 136, 1, 180, 15, 111, 92]),
                                Address::from_public_key_bytes(&[129, 57, 119, 14, 168, 125, 23, 95, 86, 163, 84, 102, 195, 76, 126, 204, 203, 141, 138, 145, 180, 238, 55, 162, 93, 246, 15, 91, 143, 201, 179, 148])
                            ];
                            let longest_chain = blockchain.lock().unwrap().all_blocks_in_longest_chain().clone();
                            let block_hash = longest_chain.get(block).unwrap();
                            let blk_state = block_state_map.lock().unwrap().block_state_map.get(block_hash).unwrap().clone();
                            let mut result: Vec<String> = Vec::new();
                            for account in accounts {
                                if blk_state.contains_key(&account) {
                                    let (nonce, balance) = blk_state.get(&account).unwrap();
                                    let s = String::from("(".to_owned() + account.to_string().as_str() + ", " + &nonce.to_string() + ", " + &balance.to_string() + ")");
                                    result.push(s);
                                }
                            }
                            respond_json!(req, result);
                        }
                        _ => {
                            let content_type =
                                "Content-Type: application/json".parse::<Header>().unwrap();
                            let payload = ApiResponse {
                                success: false,
                                message: "endpoint not found".to_string(),
                            };
                            let resp = Response::from_string(
                                serde_json::to_string_pretty(&payload).unwrap(),
                            )
                            .with_header(content_type)
                            .with_status_code(404);
                            req.respond(resp).unwrap();
                        }
                    }
                });
            }
        });
        info!("API server listening at {}", &addr);
    }
}
