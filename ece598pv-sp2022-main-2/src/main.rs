#[cfg(test)]
#[macro_use]
extern crate hex_literal;

pub mod api;
pub mod blockchain;
pub mod types;
pub mod miner;
pub mod network;
pub mod transaction_generator;

use blockchain::Blockchain;
use clap::clap_app;
use miner::Mempool;
use ring::signature::KeyPair;
use smol::channel;
use log::{error, info};
use api::Server as ApiServer;
use types::transaction::ICO;
use std::net;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use crate::types::address::Address;
use crate::types::block::BlockState;
use crate::types::key_pair::given;

fn main() {
    // parse command line arguments
    let matches = clap_app!(Bitcoin =>
     (version: "0.1")
     (about: "Bitcoin client")
     (@arg verbose: -v ... "Increases the verbosity of logging")
     (@arg peer_addr: --p2p [ADDR] default_value("127.0.0.1:6000") "Sets the IP address and the port of the P2P server")
     (@arg api_addr: --api [ADDR] default_value("127.0.0.1:7000") "Sets the IP address and the port of the API server")
     (@arg known_peer: -c --connect ... [PEER] "Sets the peers to connect to at start")
     (@arg p2p_workers: --("p2p-workers") [INT] default_value("4") "Sets the number of worker threads for P2P server")
    )
    .get_matches();

    // init logger
    let verbosity = matches.occurrences_of("verbose") as usize;
    stderrlog::new().verbosity(verbosity).init().unwrap();
    let blockchain = Blockchain::new();
    let blockchain = Arc::new(Mutex::new(blockchain));
    let mempool = Mempool::new();
    let mempool = Arc::new(Mutex::new(mempool));
    // create 3 key-pairs for nodes
    let pair0 = given(&[0; 32]);
    let key0: &[u8] = pair0.public_key().as_ref();
    let account0 = Address::from_public_key_bytes(&key0);
    let pair1 = given(&[1; 32]);
    let key1: &[u8] = pair1.public_key().as_ref();
    let account1 = Address::from_public_key_bytes(&key1);
    let pair2 = given(&[2; 32]);
    let key2: &[u8] = pair2.public_key().as_ref();
    let account2 = Address::from_public_key_bytes(&key2);
    let ico = Arc::new(Mutex::new(ICO::new(&key0)));
    let block_state_map = Arc::new(Mutex::new(BlockState::new()));
    let genesis_hash = blockchain.lock().unwrap().tip();
    //record genesis block's state
    block_state_map.lock().unwrap().block_state_map.insert(genesis_hash, ico.lock().unwrap().state.clone());

    // parse p2p server address
    let p2p_addr = matches
        .value_of("peer_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P server address: {}", e);
            process::exit(1);
        });
    let address_to_use = p2p_addr.port() % 10;

    // parse api server address
    let api_addr = matches
        .value_of("api_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing API server address: {}", e);
            process::exit(1);
        });

    // create channels between server and worker
    let (msg_tx, msg_rx) = channel::bounded(10000);

    // start the p2p server
    let (server_ctx, server) = network::server::new(p2p_addr, msg_tx).unwrap();
    server_ctx.start().unwrap();

    // start the worker
    let p2p_workers = matches
        .value_of("p2p_workers")
        .unwrap()
        .parse::<usize>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P workers: {}", e);
            process::exit(1);
        });
    let worker_ctx = network::worker::Worker::new(
        p2p_workers,
        msg_rx,
        &server,
        &blockchain,
        &mempool,
        &block_state_map
    );
    worker_ctx.start();

    // start generating transactions BEFORE miner
    let mut chosen_address = account0;
    let mut chosen_keypair = pair0;
    let mut receiver_addresses = [account1, account2];
    if address_to_use == 1 {
        chosen_address = account1;
        chosen_keypair = pair1;
        receiver_addresses = [account0, account2];
    } else if address_to_use == 2 {
        chosen_address = account2;
        chosen_keypair = pair2;
        receiver_addresses = [account0, account1];
    }
    let (generator_ctx, generator, finished_tx_chan) =
        transaction_generator::new(&blockchain, &chosen_address, chosen_keypair, &block_state_map, receiver_addresses.clone());
    let generator_worker_ctx = transaction_generator::worker::Worker::new(&server, finished_tx_chan, &mempool);
    generator_ctx.start();
    generator_worker_ctx.start();

    // start the miner
    let (miner_ctx, miner, finished_block_chan) = miner::new(&blockchain, &mempool, &block_state_map);
    let miner_worker_ctx = miner::worker::Worker::new(&server, finished_block_chan, &blockchain);
    miner_ctx.start();
    miner_worker_ctx.start();

    // connect to known peers
    if let Some(known_peers) = matches.values_of("known_peer") {
        let known_peers: Vec<String> = known_peers.map(|x| x.to_owned()).collect();
        let server = server.clone();
        thread::spawn(move || {
            for peer in known_peers {
                loop {
                    let addr = match peer.parse::<net::SocketAddr>() {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Error parsing peer address {}: {}", &peer, e);
                            break;
                        }
                    };
                    match server.connect(addr) {
                        Ok(_) => {
                            info!("Connected to outgoing peer {}", &addr);
                            break;
                        }
                        Err(e) => {
                            error!(
                                "Error connecting to peer {}, retrying in one second: {}",
                                addr, e
                            );
                            thread::sleep(time::Duration::from_millis(1000));
                            continue;
                        }
                    }
                }
            }
        });
    }

    // start the API server
    ApiServer::start(
        api_addr,
        &miner,
        &generator,
        &server,
        &blockchain,
        &block_state_map
    );

    loop {
        std::thread::park();
    }
}
