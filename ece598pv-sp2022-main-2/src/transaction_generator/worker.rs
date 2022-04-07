use crossbeam::channel::{Receiver};
use log::{info};
use crate::miner::Mempool;
use crate::network::message::Message;
use crate::types::hash::H256;
use crate::types::transaction::SignedTransaction;
use crate::types::{hash::Hashable};
use crate::network::server::Handle as ServerHandle;
use std::thread;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Worker {
    server: ServerHandle,
    finished_tx_chan: Receiver<SignedTransaction>,
    mempool: Arc<Mutex<Mempool>>
}

impl Worker {
    pub fn new(
        server: &ServerHandle,
        finished_tx_chan: Receiver<SignedTransaction>,
        mempool: &Arc<Mutex<Mempool>>
    ) -> Self {
        Self {
            server: server.clone(),
            finished_tx_chan,
            mempool: Arc::clone(mempool)
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("transaction-generator-worker".to_string())
            .spawn(move || {
                self.transaction_generator_loop();
            })
            .unwrap();
        info!("Transaction generator initialized into paused mode");
    }

    fn transaction_generator_loop(&self) {
        loop {
            let _transaction = self.finished_tx_chan.recv().expect("Received finished transaction error");
            let mut mempool_ = self.mempool.lock().unwrap();
            mempool_.insert(&_transaction);
        
            let mut tx_to_send = Vec::<H256>::new();
            tx_to_send.push(_transaction.hash());
            self.server.broadcast(Message::NewTransactionHashes(tx_to_send));
        }
    }
}
