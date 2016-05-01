use std::fmt;
use std::mem::transmute;
use std::sync::PoisonError;

use ::message::RawMessage;


#[derive(Debug)]
pub enum WorkflowError {
    /// Message with unknown code
    UnknownCode,
    ConnectionError,
    ProtocolError(String),
    Exception(String),
}


#[derive(Debug)]
pub enum Workflow {
    Continue,
    Terminate(WorkflowError),
    SwitchProtocol(usize)
}


#[derive(PartialEq, PartialOrd, Debug)]
pub struct ProtocolVersion(pub u8, pub u8, pub u16);


pub trait Protocol {
    fn flow(&self, raw_message: RawMessage) -> Workflow;
}

// --------------------------------------------------------------------------------------------------------------------

impl <T> From<PoisonError<T>> for WorkflowError {
    fn from(error: PoisonError<T>) -> WorkflowError {
        WorkflowError::Exception(format!("{:?}", error))
    }
}


impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return format!("{:?}", self).fmt(f);
    }
}


impl ProtocolVersion {
    pub fn new (v0: u8, v1: u8, v2: u16) -> ProtocolVersion {
        ProtocolVersion(v0, v1, v2)
    }

    pub fn export(&self) -> Vec<u8> {
        [
            &[ self.0, self.1, ],
            &(unsafe { transmute::<u16, [u8; 2]>(self.2.to_be()) })[..]
        ].concat()
    }
}
