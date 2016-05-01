use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use protocol::message::Message;


pub type StreamMessage = Option<Message>;
pub type StreamSender = Arc<Mutex<Sender<StreamMessage>>>;


pub fn send_message(tx: &StreamSender, message: StreamMessage) {
    let locked = tx.lock().unwrap();
    locked.send(message).unwrap();
}


