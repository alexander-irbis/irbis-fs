
use std::str::Utf8Error;

use protocol::message::{RawMessage, RawMessageBody};
use protocol::serde::{Encoder, Encode, Parse, Parser, ParserError};

use ::types::{ContentId, TaskId};


#[derive(Debug, PartialEq)]
pub enum ParseError {
    /// Message with unknown code
    UnknownCode,
    ParseError(String),
    BadProtocol,
}


#[derive(Debug)]
pub enum ClientMessage {
    GetInfo(CGetInfo),
    CopyFrom(CCopyFrom),
}


#[derive(Debug)]
pub enum ServerMessage {
    Info(SInfo),
    CopyFrom(SCopyFrom),
    Reject(SReject),
    Error(SError),
}

// --------------------------------------------------------------------------------------------------------------------

pub const MC_GET_INFO: u8 = 1;
pub const MC_COPY_FROM: u8 = 2;

#[derive(Debug)]
pub struct CGetInfo {
    pub task_id: TaskId,
}

#[derive(Debug)]
pub struct CCopyFrom {
    pub task_id: TaskId,
    pub uri: String,
}


pub const MS_INFO: u8 = 1;
pub const MS_COPY_FROM: u8 = 2;

pub const MS_REJECT: u8 = 254;
pub const MS_ERROR: u8 = 255;

#[derive(Debug)]
pub struct SInfo {
    pub task_id: TaskId,
    pub pid: u32,
    pub arch_bits: u16,
    pub os: String,
}

#[derive(Debug)]
pub enum SCopyFromState {
    Progress(u8),
    Complete(ContentId),
}

#[derive(Debug)]
pub struct SCopyFrom {
    pub task_id: TaskId,
    pub state: SCopyFromState,
}

#[derive(Debug)]
pub struct SReject {
    pub task_id: TaskId,
    pub reason: String,
}

#[derive(Debug)]
pub struct SError {
    pub task_id: TaskId,
    pub message: String,
}


// --------------------------------------------------------------------------------------------------------------------


impl From<Utf8Error> for ParseError {
    fn from(err: Utf8Error) -> Self {
        ParseError::ParseError(format!("{:?}", err))
    }
}


impl From<ParserError> for ParseError {
    fn from(err: ParserError) -> Self {
        ParseError::ParseError(format!("{:?}", err))
    }
}


// --------------------------------------------------------------------------------------------------------------------


//impl RawCodec for ClientMessage {
impl ClientMessage {
    pub fn encode(self) -> RawMessage {
        match self {
            ClientMessage::GetInfo(m)   => RawMessage::new(MC_GET_INFO, m.encode()),
            ClientMessage::CopyFrom(m)  => RawMessage::new(MC_COPY_FROM, m.encode()),
        }
    }

    pub fn parse(raw_message: RawMessage) -> Result<ClientMessage, ParseError> {
        match raw_message.mtype {
            MC_GET_INFO   => Ok(try!(CGetInfo::parse(raw_message.body))),
            MC_COPY_FROM  => Ok(try!(CCopyFrom::parse(raw_message.body))),
            _           => Err(ParseError::UnknownCode)
        }
    }
}


//impl RawCodec for ServerMessage {
impl ServerMessage {
    pub fn encode(self) -> RawMessage {
        match self {
            ServerMessage::Info(m)      => RawMessage::new(MS_INFO, m.encode()),
            ServerMessage::CopyFrom(m)  => RawMessage::new(MS_COPY_FROM, m.encode()),
            ServerMessage::Reject(m)    => RawMessage::new(MS_REJECT, m.encode()),
            ServerMessage::Error(m)     => RawMessage::new(MS_ERROR, m.encode()),
        }
    }

    pub fn parse(raw_message: RawMessage) -> Result<ServerMessage, ParseError> {
        match raw_message.mtype {
            MS_INFO      => Ok(try!(SInfo::parse(raw_message.body))),
            MS_COPY_FROM => Ok(try!(SCopyFrom::parse(raw_message.body))),
            MS_REJECT    => Ok(try!(SReject::parse(raw_message.body))),
            MS_ERROR     => Ok(try!(SError::parse(raw_message.body))),
            _            => Err(ParseError::UnknownCode)
        }
    }

    pub fn get_task_id(&self) -> TaskId {
        match *self {
            ServerMessage::Info(ref m)      => m.task_id,
            ServerMessage::CopyFrom(ref m)  => m.task_id,
            ServerMessage::Reject(ref m)    => m.task_id,
            ServerMessage::Error(ref m)     => m.task_id,
        }
    }
}


// --------------------------------------------------------------------------------------------------------------------


impl CGetInfo {
    pub fn create(task_id: TaskId) -> ClientMessage {
        ClientMessage::GetInfo(CGetInfo{task_id: task_id})
    }

    pub fn encode(&self) -> RawMessageBody {
        let mut encode = Encoder::new();

        encode += self.task_id;

        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ClientMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {

                let mut input = Parser::new(v);

                let task_id     = u64::parse_from(&mut input)?;

                input.complete()?;

                Ok(CGetInfo::create(task_id))
            },
            _ => Err(ParseError::BadProtocol)
        }
    }
}


impl CCopyFrom {
    pub fn create(task_id: TaskId, uri: String) -> ClientMessage {
        ClientMessage::CopyFrom(CCopyFrom{ task_id: task_id, uri: uri })
    }

    pub fn encode(self) -> RawMessageBody {
        let mut encode = Encoder::new();

        encode += self.task_id;
        encode += self.uri;

        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ClientMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {
                let mut input = Parser::new(v);

                let task_id     = u64::parse_from(&mut input)?;
                let uri         = String::parse_from(&mut input)?;

                input.complete()?;

                Ok(CCopyFrom::create(task_id, uri))
            },
            _ => Err(ParseError::BadProtocol)
        }
    }
}


// --------------------------------------------------------------------------------------------------------------------


impl SInfo {
    pub fn create(task_id: TaskId, pid: u32, arch_bits: u16, os: String) -> ServerMessage {
        ServerMessage::Info(SInfo {
            task_id: task_id,
            pid: pid,
            arch_bits: arch_bits,
            os: os,
        })
    }

    pub fn encode(self) -> RawMessageBody {

        let mut encode = Encoder::new();

        encode += self.task_id;
        encode += self.pid;
        encode += self.arch_bits;
        encode += self.os;

        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ServerMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {

                let mut input = Parser::new(v);

                let task_id     = u64::parse_from(&mut input)?;
                let pid         = u32::parse_from(&mut input)?;
                let arch_bits   = u16::parse_from(&mut input)?;
                let os          = String::parse_from(&mut input)?;

                input.complete()?;

                Ok(SInfo::create(task_id, pid, arch_bits, os))
            },
            _ => Err(ParseError::BadProtocol)
        }
    }
}


// --------------------------------------------------------------------------------------------------------------------


impl Encode for SCopyFromState {
    fn encode(self) -> Vec<u8> {
        let mut encoder = Encoder::new();

        match self {
            SCopyFromState::Complete(content_id) => {
                encoder += 0u8;
                encoder += content_id;
            },
            SCopyFromState::Progress(progress) => {
                encoder += 1u8;
                encoder += progress;
            },
        }

        encoder.complete()
    }
}

impl Parse for SCopyFromState {
    fn parse_from(input: &mut Parser) -> Result<SCopyFromState, ParserError> {
        Ok(match u8::parse_from(input)? {
            0 => SCopyFromState::Complete(ContentId::parse_from(input)?),
            1 => SCopyFromState::Progress(u8::parse_from(input)?),
            _ => unreachable!()
        })
    }
}


impl SCopyFrom {
    pub fn create(task_id: TaskId, state: SCopyFromState) -> ServerMessage {
        ServerMessage::CopyFrom(SCopyFrom {
            task_id: task_id,
            state: state,
        })
    }

    pub fn encode(self) -> RawMessageBody {

        let mut encode = Encoder::new();

        encode += self.task_id;
        encode += self.state;

        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ServerMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {

                let mut input = Parser::new(v);

                let task_id     = u64::parse_from(&mut input)?;
                let state       = SCopyFromState::parse_from(&mut input)?;

                input.complete()?;

                Ok(SCopyFrom::create(task_id, state))
            },
            _ => Err(ParseError::BadProtocol)
        }
    }
}


impl SReject {
    pub fn create(task_id: TaskId, reason: String) -> ServerMessage {
        ServerMessage::Reject(SReject{ task_id: task_id, reason: reason })
    }

    pub fn encode(self) -> RawMessageBody {

        let mut encode = Encoder::new();

        encode += self.task_id;
        encode += self.reason;

        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ServerMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {

                let mut input = Parser::new(v);

                let task_id     = u64::parse_from(&mut input)?;
                let reason      = String::parse_from(&mut input)?;

                input.complete()?;

                Ok(SReject::create(task_id, reason))
            },
            _ => Err(ParseError::BadProtocol)
        }
    }
}

impl SError {
    pub fn create(task_id: TaskId, message: String) -> ServerMessage {
        ServerMessage::Error(SError{ task_id: task_id, message: message })
    }

    pub fn encode(self) -> RawMessageBody {

        let mut encode = Encoder::new();

        encode += self.task_id;
        encode += self.message;

        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ServerMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {

                let mut input = Parser::new(v);

                let task_id     = u64::parse_from(&mut input)?;
                let message     = String::parse_from(&mut input)?;

                input.complete()?;

                Ok(SError::create(task_id, message))
            },
            _ => Err(ParseError::BadProtocol)
        }
    }
}



//impl SInfo {
//    pub fn create(task_id: TaskId, pid: u32, arch_bits: u16, os: String) -> ServerMessage {
//        ServerMessage::Info(SInfo {
//            task_id: task_id,
//            pid: pid,
//            arch_bits: arch_bits,
//            os: os,
//        })
//    }
//
//    pub fn encode(&self) -> RawMessageBody {
//        let os_len = self.os.len() as u16;
//        let data: Vec<u8> = [
//            &(unsafe { transmute::<TaskId, [u8; 8]>(self.task_id.to_be()) })[..],
//            &(unsafe { transmute::<u32, [u8; 4]>(self.pid.to_be()) })[..],
//            &(unsafe { transmute::<u16, [u8; 2]>(self.arch_bits.to_be()) })[..],
//            &(unsafe { transmute::<u16, [u8; 2]>(os_len.to_be()) })[..],
//            self.os.as_bytes()
//        ].concat();
//        RawMessageBody::Binary(data)
//    }
//
//    pub fn parse(body: RawMessageBody) -> Result<ServerMessage, ParseError> {
//        match body {
//            RawMessageBody::Binary(v) => {
//
//                let mut task_id = [0; 8];
//                task_id.copy_from_slice(&v[0..8]);
//                let task_id = TaskId::from_be(unsafe { transmute::<[u8; 8], TaskId>(task_id) });
//
//                let mut pid = [0; 4];
//                pid.copy_from_slice(&v[8..8+4]);
//                let pid = u32::from_be(unsafe { transmute::<[u8; 4], u32>(pid) });
//
//                let arch_bits: u16 = (v[12] as u16) * 256 + (v[13] as u16);
//
//                let os_len: usize = (v[14] as usize) * 256 + (v[15] as usize);
//                let os = try!(from_utf8(&v[16 .. 16 + os_len])).to_owned();
//
//                Ok(SInfo::create(task_id, pid, arch_bits, os))
//            },
//            _ => Err(ParseError::BadProtocol)
//        }
//    }
//}
