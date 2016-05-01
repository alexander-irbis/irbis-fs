use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, channel};
use std::thread;

use protocol::stream::Stream;
use protocol::message::{Message, RawMessage, Reader, ReadError, ReadFlow};

use ::connection::{StreamSender, StreamMessage};
use ::proto::auth::client::{AuthProtocol, AuthError};
use ::proto::content::client::{ContentProtocol, ContentInterface};


#[cfg(feature = "dev")]
static mut depth: u32 = 0;


#[derive(Debug)]
pub struct Connection {
    pub stream: Stream,
    writer_tx: StreamSender,
}


impl Connection  {
    pub fn connect(stream: Stream) -> Result<ContentInterface, AuthError> {
        #![allow(unused_must_use)]
        let (tx, rx) = channel::<StreamMessage>();

        let connection = Connection {
            stream: stream,
            writer_tx: Arc::new(Mutex::new(tx)),
        };

        connection.create_writer_thread(rx);

        let mut protocol = AuthProtocol::new(connection);
        match protocol.auth() {
            Ok(client_id) => {
                Ok(ContentInterface::new(Arc::new(ContentProtocol::new(protocol.connection, client_id))))
            },
            Err(err) => Err(err)
        }
    }

    pub fn sender(&self) -> StreamSender {
        self.writer_tx.clone()
    }

    pub fn send_message(&self, message: StreamMessage) {
        ::connection::send_message(&self.writer_tx, message);
    }

    /// Creates a thread that writes into the server stream each message received
    fn create_writer_thread(&self, rx: Receiver<Option<Message>>) {
        let mut stream = self.stream.try_clone().unwrap();
        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(m) => match m {
                        Some(msg) => {
//                            trace!(" * before write {:?}", msg);
                            match stream.write(&*msg.as_bytes()) {
                                Ok(_) => (),
                                Err(e) => {
                                    warn!("Error writing to server: {:?}", e);
                                    break;
                                },
                            }
                        },
                        None => break,
                    },
                    Err(_) => break,
                };
            }
//            stream.shutdown();
        });
    }

    #[cfg_attr(feature = "dev", trace)]
    pub fn read(&self) -> Result<RawMessage, ReadError> {
        let mut stream = self.stream.try_clone().unwrap();
        Self::_read(&mut stream)
    }


    #[cfg_attr(feature = "dev", trace)]
    pub fn _read(stream: &mut Stream) -> Result<RawMessage, ReadError> {
        let mut reader = Reader::new();
        'read_message: loop {
            match reader.read(stream) {
                ReadFlow::Error(err)    => return Err(err),
                ReadFlow::Incomplete    => continue 'read_message,
                ReadFlow::Complete      => match reader.to_message() {
                    Ok(message) => return Ok(message),
                    Err(err)    => {
                        return Err(ReadError::Fatal(format!("Parse error: {:?}", err)))
                    }
                },
            };
        }
    }
}
