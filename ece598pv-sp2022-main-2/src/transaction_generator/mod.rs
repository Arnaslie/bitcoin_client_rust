pub mod worker;

use log::info;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use ring::signature::Ed25519KeyPair;
use std::time;

use std::thread;

use crate::types::address::Address;
use crate::blockchain::{Blockchain};
use crate::types::block::BlockState;
use crate::types::transaction::{SignedTransaction, Transaction, sign};
use ring::signature::{KeyPair};
use std::sync::{Arc, Mutex};
use rand::Rng;
enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Update, // update the block in mining, it may due to new blockchain tip or new transaction
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    finished_tx_chan: Sender<SignedTransaction>,
    blockchain: Arc<Mutex<Blockchain>>,
    address: Address,
    keypair: Ed25519KeyPair,
    block_state_map: Arc<Mutex<BlockState>>,
    receiver_addresses: [Address; 2]
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the transaction thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(blockchain: &Arc<Mutex<Blockchain>>,
           address: &Address,
           keypair: Ed25519KeyPair,
           block_state_map: &Arc<Mutex<BlockState>>,
           receiver_addresses: [Address; 2]) -> (Context, Handle, Receiver<SignedTransaction>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (finished_tx_sender, finished_tx_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        finished_tx_chan: finished_tx_sender,
        blockchain: Arc::clone(blockchain),
        address: address.clone(),
        keypair: keypair,
        block_state_map: Arc::clone(block_state_map),
        receiver_addresses: receiver_addresses
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle, finished_tx_receiver)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, theta: u64) {
        self.control_chan
            .send(ControlSignal::Start(theta))
            .unwrap();
    }

    pub fn update(&self) {
        self.control_chan.send(ControlSignal::Update).unwrap();
    }
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("transaction_generator".to_string())
            .spawn(move || {
                self.transaction_generator_loop();
            })
            .unwrap();
        info!("Transaction generator initialized into paused mode");
    }

    fn transaction_generator_loop(&mut self) {
        let mut receiver_index = 0;
        // main transaction_generator loop
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            info!("Transaction generator shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            info!("Transaction generator starting in continuous mode with theta {}", i);
                            self.operating_state = OperatingState::Run(i);
                        }
                        ControlSignal::Update => {
                            // in paused state, don't need to update
                        }
                    };
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        match signal {
                            ControlSignal::Exit => {
                                info!("Transaction generator shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Transaction generator starting in continuous mode with theta {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                                unimplemented!()
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Transaction generator control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            //generate valid transactions based off current tip state
            let tip = self.blockchain.lock().unwrap().tip().clone();
            let tip_state = self.block_state_map.lock().unwrap().block_state_map.get(&tip).unwrap().clone();
            let receiver = self.receiver_addresses[receiver_index];
            let sender_balance; 
            if tip_state.contains_key(&self.address) {
                sender_balance = tip_state.get(&self.address).unwrap();
            } else {
                sender_balance = &(0, 0);
            }
            if sender_balance.1 == 0 {
                continue
            }
            let mut rng = rand::thread_rng();
            let mut val = sender_balance.1 / 2;
            if val == 0 { val = 1; }
            let nonce;
            if tip_state.contains_key(&self.address) {
                nonce = tip_state.get(&self.address).unwrap().0;
            } else {
                nonce = 0;
            }
            let tx = Transaction {
                sender: self.address,
                receiver: receiver,
                value: rng.gen_range(1..val),
                account_nonce: nonce + 1
            };
            let key_pair = &self.keypair;
            let signature_ = sign(&tx, &key_pair);
            let signed_tx = SignedTransaction {
                transaction: tx,
                signature: signature_.as_ref().to_vec(),
                public_key: key_pair.public_key().as_ref().to_vec()
            };
            self.finished_tx_chan.send(signed_tx.clone()).expect("Send finished transaction error");
            if receiver_index == 0 { receiver_index = 1; }
            else { receiver_index = 0; }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}
