use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, channel};
use std::thread;

use protocol::stream::Stream;
use protocol::message::{Message, RawMessage, Reader, ReadError, ReadFlow};
use protocol::workflow::{Protocol, Workflow};

use ::connection::{StreamSender, StreamMessage};
use ::proto::auth::server::AuthProtocol;
use ::proto::content::server::{ContentProtocol, ContentConfig};

use super::database::Database;


/// A client connection
pub struct Connection {
    /// The socket connection
    stream: Stream,
    /// A reference to the database
    db: Arc<Mutex<Database>>,
    /// The client unique identifier
    id: usize,
}


impl Connection {
    /// Creates a new client
    pub fn new(stream: Stream, db: Arc<Mutex<Database>>, id: usize) -> Connection {
        return Connection {
            stream: stream,
            db: db,
            id: id,
        }
    }

    /// Creates a thread that writes into the client stream each response received
    fn create_writer_thread(&self, rx: Receiver<Option<Message>>) {
        let mut stream = self.stream.try_clone().unwrap();
        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(m) => match m {
                        Some(msg) => match stream.write(&*msg.as_bytes()) {
                            Ok(_) => (),
                            Err(e) => warn!("Error writing to client: {:?}", e),
                        },
                        None => break,
                    },
                    Err(_) => break,
                };
            }
        });
    }

    /// Runs all clients commands. The function loops until the client
    /// disconnects.
    pub fn run(&mut self) {
        #![allow(unused_must_use)]
        let (stream_tx, rx) = channel::<StreamMessage>();
        self.create_writer_thread(rx);

        let stream_tx = Arc::new(Mutex::new(stream_tx));

        if self.run_auth(stream_tx.clone()).is_ok() {
            self.run_content(stream_tx);
        };
    }

    fn run_auth(&mut self, stream_tx: StreamSender) -> Result<(), ()> {

        let protocol = AuthProtocol::new(stream_tx, self.id);

        info!(">>::  New connection. Starting auth");

        'iter_messages: loop {
            let message = match self.read() {
                Ok(message) => message,
                Err(error) => {
                    error!("Auth stage failed: {:?}", error);
                    return Err(());
                }
            };

            match protocol.flow(message) {
                Workflow::Continue          => continue 'iter_messages,
                Workflow::Terminate(m)      => {
                    info!("Terminated: {}", m);
                    return Err(());
                },
                Workflow::SwitchProtocol(_) => {
                    info!("Switch Protocol to: ContentProtocol");
                    break 'iter_messages;
                }
            };
        }
        Ok(())
    }

    fn run_content(&mut self, stream_tx: StreamSender) {

        let protocol: ContentProtocol = ContentProtocol::new(stream_tx, self.id, self.db.clone());

        info!("  ::  Auth Ok. Switched to ContentProtocol");

        'iter_messages: loop {
            let message = match self.read() {
                Ok(message) => message,
                Err(error) => {
                    info!("Read error: {:?}", error);
                    break 'iter_messages;
                }
            };

            match protocol.flow(message) {
                Workflow::Continue          => (),
                Workflow::Terminate(m)      => {
                    info!("Terminated: {}", m);
                    break 'iter_messages;
                },
                Workflow::SwitchProtocol(_) => {
                    unreachable!();
                }
            };
        }

        info!("<<::  Hangup");
    }

    #[cfg_attr(feature = "dev", trace)]
    pub fn read(&mut self) -> Result<RawMessage, ReadError> {
        let mut reader = Reader::new();

        'read_message: loop {
            match reader.read(&mut self.stream) {
                ReadFlow::Error(err)    => return Err(err),
                ReadFlow::Incomplete    => continue 'read_message,
                ReadFlow::Complete      => match reader.to_message() {
                    Ok(message) => return Ok(message),
                    Err(err)    => {
                        info!("Parse error: {:?}", err);
                        return Err(ReadError::Fatal(format!("Parse error: {:?}", err)))
                    }
                },
            };
        }
    }
}
