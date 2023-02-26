use crossbeam::channel::{Receiver};
use log::{info, debug};
use crate::network::message::Message;
use crate::types::hash::H256;
use crate::types::{block::Block, hash::Hashable};
use crate::network::server::Handle as ServerHandle;
use std::thread;
use crate::blockchain::Blockchain;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Worker {
    server: ServerHandle,
    finished_block_chan: Receiver<Block>,
    blockchain: Arc<Mutex<Blockchain>>,
}

impl Worker {
    pub fn new(
        server: &ServerHandle,
        finished_block_chan: Receiver<Block>,
        blockchain: &Arc<Mutex<Blockchain>>,
    ) -> Self {
        Self {
            server: server.clone(),
            finished_block_chan,
            blockchain: Arc::clone(blockchain),
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("miner-worker".to_string())
            .spawn(move || {
                self.worker_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn worker_loop(&self) {
        loop {
            let _block = self.finished_block_chan.recv().expect("Receive finished block error");
            let mut blockchain_ = self.blockchain.lock().unwrap();
            blockchain_.insert(&_block);
            let mut block_to_send = Vec::<H256>::new();
            block_to_send.push(_block.hash());
            debug!("SENDING BLOCK: {}", _block.hash());
            self.server.broadcast(Message::NewBlockHashes(block_to_send));
        }
    }
}
