use std::cmp::{max, min};
use std::error::Error;
use std::fmt;
use std::io::Read;
use std::mem::transmute;
use std::str::{from_utf8, Utf8Error};

use ::stream::Stream;


#[cfg(feature = "dev")]
static mut depth: u32 = 0;


pub const BT_BINARY: u8 = 0;
pub const BT_TEXT  : u8 = 1;
pub const BT_JSON  : u8 = 2;

pub const BODY_SIZE_LIMIT: usize = 16_777_216;
pub const HEADER_SIZE: usize = 8;


#[derive(Debug, Clone)]
pub enum RawMessageBody {
    Binary(Vec<u8>),
    Text(String),
    JSON(String),
}


#[derive(Debug, Clone)]
pub struct RawMessage {
    pub mtype: u8,
    pub body: RawMessageBody
}


#[derive(Debug)]
pub enum ReadFlow {
    Complete,
    /// The received buffer is valid but needs more data
    Incomplete,
    Error(ReadError),
}


/// Error parsing
#[derive(Debug, PartialEq)]
pub enum ReadError {
//    /// Nonfatal error, drop task if started
//    NonFatal(String),
    /// Fatal error, drop connection and all started tasks
    Fatal(String),
    /// Lost connection, drop all started tasks
    ConnectionError(String),
}


#[derive(Debug)]
pub enum ParseError {
    Utf8(Utf8Error),
    UnknownContentType,
}


#[derive(Debug)]
pub enum EncodeError {
    TooLong,
}


#[derive(Debug)]
pub struct Message {
    data: Vec<u8>
}


// --------------------------------------------------------------------------------------------------------------------


impl ReadError {
//    pub fn is_fatal(&self) -> bool {
//        match *self {
//            ReadError::NonFatal => false,
//            _ => true,
//        }
//    }

    fn response_string(&self) -> String {
        match *self {
//            ReadError::NonFatal(ref s) => format!("NonFatal protocol error: {}", s),
            ReadError::Fatal(ref s) => format!("Fatal protocol error: {}", s),
            ReadError::ConnectionError(ref s) => format!("Connection error: {}", s),
        }
    }
}


impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return self.response_string().fmt(f);
    }
}


impl Error for ReadError {
    fn description(&self) -> &str {
        match *self {
//            ReadError::NonFatal(_) => "Nonfatal protocol error",
            ReadError::Fatal(_) => "Protocol error",
            ReadError::ConnectionError(_) => "Connection error",
        }
    }

    fn cause(&self) -> Option<&Error> { None }
}


// --------------------------------------------------------------------------------------------------------------------


/// A stream reader
pub struct Reader {
    header: Vec<u8>,
    body: Vec<u8>,
    position: usize,
    size: Option<usize>,
}

impl Reader {
    pub fn new() -> Reader {
        Reader {
            header: vec![0; HEADER_SIZE],
            body: [0; 0].to_vec(),
            position: 0,
            size: None
        }
    }

    #[cfg_attr(feature = "dev", trace)]
    pub fn read(&mut self, stream: &mut Stream) -> ReadFlow {
        let len = match stream.read(match self.size {
            None        => &mut self.header[self.position .. HEADER_SIZE],
            Some(size)  => &mut self.body[self.position .. size],
        }) {
            Ok(r) => r,
            Err(err) => {
                return ReadFlow::Error(ReadError::ConnectionError(format!("Reading from client: {:?}", err)));
            },
        };
        self.position += len;

        if len == 0 {
            return match self.is_complete() {
                true    => ReadFlow::Complete,
                false   => ReadFlow::Error(ReadError::ConnectionError("Peer closed connection".to_owned())),
            };
        }

        if self.size.is_none() && self.position == HEADER_SIZE {
//            trace!("    ==  {:?}; {:?}", self.size, self.position);
            let size: usize = self.header[4..8].iter().fold(0usize, |a, x| a * 256 + (*x as usize));
            if size > BODY_SIZE_LIMIT {
                return ReadFlow::Error(ReadError::Fatal(format!("message size ({}) exceed limit 16 MB", size)))
            }
            self.size = Some(size);
            self.position = 0;
            self.body = vec![0; size];
        }

        match self.is_complete() {
            true    => ReadFlow::Complete,
            false   => ReadFlow::Incomplete,
        }
    }

    pub fn is_complete(&self) -> bool {
        match self.size {
            None        => false,
            Some(size)  => size == self.position,
        }
    }

    pub fn to_message(&self) -> Result<RawMessage, ParseError> {
        let (mtype, body_type) = (self.header[2], self.header[3]);

        let body: RawMessageBody = match body_type {
            BT_BINARY => RawMessageBody::Binary(self.body.clone()),
            BT_TEXT => RawMessageBody::Text(
                match from_utf8(&self.body[..]) {
                    Ok(r) => r.to_owned(),
                    Err(err) => return Err(ParseError::Utf8(err)),
                }
            ),
            BT_JSON => RawMessageBody::JSON(
                match from_utf8(&self.body[..]) {
                    Ok(r) => r.to_owned(),
                    Err(err) => return Err(ParseError::Utf8(err)),
                }
            ),
            _ => return Err(ParseError::UnknownContentType),
        };
        Ok(RawMessage::new(mtype, body))
    }
}


impl fmt::Debug for Reader {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        try!(f.write_str(&format!("Reader({} => {}); header: {:?}; body last 16 bytes: {:?}",
            match self.size {
                None => format!("header({})", 8),
                Some(size) => format!("body({})", size),
            },
            self.position,
            &self.header[..],
            &self.body[min(self.position, max(self.body.len(), 16) - 16) .. min(self.position + 16, self.body.len())]
        )[..]));
        Ok(())
    }
}


// --------------------------------------------------------------------------------------------------------------------


impl RawMessage {
    pub fn new(mtype: u8, body: RawMessageBody) -> RawMessage {
        RawMessage {
            mtype: mtype,
            body: body
        }
    }
}


// --------------------------------------------------------------------------------------------------------------------


impl Message {
    pub fn from_raw(raw_message: RawMessage) -> Result<Message, EncodeError> {
        match raw_message.body {
            RawMessageBody::Binary(v) => {
                if v.len() > BODY_SIZE_LIMIT {
                    return Err(EncodeError::TooLong);
                };
                let size: [u8; 4] = unsafe { transmute((v.len() as u32).to_be()) };
                let data: Vec<u8> = [
                    // header
                    &[0, 0, raw_message.mtype, BT_BINARY][..], &size[..],
                    // body
                    &v[..]
                ].concat();
                Ok(Message{data: data})
            },
            RawMessageBody::Text(_) => { unimplemented!() },
            RawMessageBody::JSON(_) => { unimplemented!() },
//            JSON(v) => { Ok(vec![0u8; 0]) },
        }
    }

    pub fn as_bytes(&self) -> &Vec<u8> {
        &self.data
    }
}
