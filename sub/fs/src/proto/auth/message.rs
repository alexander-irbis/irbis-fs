
use std::str::Utf8Error;

use protocol::message::{RawMessage, RawMessageBody};
use protocol::serde::{Encoder, Encode, Parse, Parser, ParserError};
use protocol::workflow::ProtocolVersion;


#[derive(Debug, PartialEq)]
pub enum ParseError {
    /// Message with unknown code
    UnknownCode,
    ParseError(String),
    BadProtocol,
}


#[derive(Debug)]
pub enum ClientMessage {
    Start(CStart),
//    AuthPlain(CAuthPlain),
//    AuthHash(CAuthHash),
//    AuthSCM(CAuthSCM),
}


#[derive(Debug)]
pub enum ServerMessage {
    AuthOk(SAuthOk),
    Reject(SReject),
    Error(SError),
//    RequestAuthPlain(SRequestAuthPlain),
//    RequestAuthHash(SRequestAuthHash),
//    RequestAuthSCM(SRequestAuthSCM),
}

// --------------------------------------------------------------------------------------------------------------------

pub const SUBPROTOCOL_CODE: u8 = 0;

pub const MC_START: u8 = 0;

#[derive(Debug)]
pub struct CStart {
    pub version: ProtocolVersion,
    pub subprotocol: u8,
    pub args: Vec<u8>,
}


pub const MS_AUTH_OK: u8 = 1;
pub const MS_REJECT: u8 = 254;
pub const MS_ERROR: u8 = 255;

#[derive(Debug)]
pub struct SAuthOk{
    pub id: usize,
}

#[derive(Debug)]
pub struct SReject {
    pub reason: String,
}

#[derive(Debug)]
pub struct SError {
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
            ClientMessage::Start(m)    => RawMessage::new(MC_START, m.encode()),
        }
    }

    pub fn parse(raw_message: RawMessage) -> Result<ClientMessage, ParseError> {
        match raw_message.mtype {
            MC_START    => Ok(CStart::parse(raw_message.body)?),
            _           => Err(ParseError::UnknownCode)
        }
    }
}


//impl RawCodec for ServerMessage {
impl ServerMessage {
    pub fn encode(self) -> RawMessage {
        match self {
            ServerMessage::AuthOk(m)   => RawMessage::new(MS_AUTH_OK, m.encode()),
            ServerMessage::Reject(m)   => RawMessage::new(MS_REJECT, m.encode()),
            ServerMessage::Error(m)    => RawMessage::new(MS_ERROR, m.encode()),
        }
    }

    pub fn parse(raw_message: RawMessage) -> Result<ServerMessage, ParseError> {
        Ok(match raw_message.mtype {
            MS_AUTH_OK  => SAuthOk::parse(raw_message.body)?,
            MS_REJECT   => SReject::parse(raw_message.body)?,
            MS_ERROR    => SError::parse(raw_message.body)?,
            _           => return Err(ParseError::UnknownCode)
        })
    }
}


// --------------------------------------------------------------------------------------------------------------------


//impl RawCodec for CStart {
impl CStart {
    pub fn create(version: ProtocolVersion, subprotocol: u8, sub_args: Vec<u8>) -> ClientMessage {
        ClientMessage::Start(CStart{
            version: version,
            subprotocol: subprotocol,
            args: sub_args,
        })
    }

    pub fn encode(self) -> RawMessageBody {
        let mut encode = Encoder::new();

        encode += self.version.0;
        encode += self.version.1;
        encode += self.version.2;
        encode += self.subprotocol;
        encode += 0u8;
        encode += 0u8;
        encode += 0u8;
        // TODO self.args

        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ClientMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {

                let mut input = Parser::new(v);

                let version     = ProtocolVersion::new(
                    u8::parse_from(&mut input)?,
                    u8::parse_from(&mut input)?,
                    u16::parse_from(&mut input)?
                );
                let subprotocol = u8::parse_from(&mut input)?;

                u8::parse_from(&mut input)?;
                u16::parse_from(&mut input)?;

                input.complete()?;

                let sub_args: Vec<u8> = Vec::new();

                Ok(CStart::create(version, subprotocol, sub_args))
            },
            _ => Err(ParseError::BadProtocol)
        }
    }
}


// --------------------------------------------------------------------------------------------------------------------


impl SAuthOk {
    pub fn create(id: usize) -> ServerMessage {
        ServerMessage::AuthOk(SAuthOk{id: id})
    }

    pub fn encode(self) -> RawMessageBody {

        let mut encode = Encoder::new();
        encode += self.id;
        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ServerMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {

                let mut input = Parser::new(v);
                let id          = usize::parse_from(&mut input)?;
                input.complete()?;

                Ok(SAuthOk::create(id))
            },
            _ => Err(ParseError::BadProtocol)
        }
    }
}


impl SReject {
    pub fn create(reason: String) -> ServerMessage {
        ServerMessage::Reject(SReject{ reason: reason })
    }

    pub fn encode(self) -> RawMessageBody {

        let mut encode = Encoder::new();
        encode += self.reason;
        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ServerMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {

                let mut input = Parser::new(v);
                let reason      = String::parse_from(&mut input)?;
                input.complete()?;

                Ok(SReject::create(reason))
            },
            _ => Err(ParseError::BadProtocol),
        }
    }
}


impl SError {
    pub fn create(message: String) -> ServerMessage {
        ServerMessage::Error(SError{ message: message })
    }

    pub fn encode(self) -> RawMessageBody {

        let mut encode = Encoder::new();
        encode += self.message;
        RawMessageBody::Binary(encode.complete())
    }

    pub fn parse(body: RawMessageBody) -> Result<ServerMessage, ParseError> {
        match body {
            RawMessageBody::Binary(v) => {

                let mut input = Parser::new(v);
                let message     = String::parse_from(&mut input)?;
                input.complete()?;

                Ok(SError::create(message))
            },
            _ => Err(ParseError::BadProtocol),
        }
    }
}
