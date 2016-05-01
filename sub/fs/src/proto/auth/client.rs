use protocol::message::{Message, RawMessage, EncodeError, ReadError};
use protocol::workflow::{Protocol, Workflow, WorkflowError};

use ::client::connection::{Connection};

use super::message::{ClientMessage, ServerMessage, CStart};
use super::super::PROTOCOL_VERSION;


#[cfg(feature = "dev")]
static mut depth: u32 = 0;


// --------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub enum AuthError {
    EncodeError(EncodeError),
    ReadError(ReadError),
    WorkflowError(WorkflowError),
    InacceptableProtocol,
}


#[derive(Debug)]
pub struct AuthConfig {
    password: Option<String>,
}


#[derive(Debug)]
pub enum AuthProtocolStage {
    BeforeStart,
    NeedAuth,
    Ok
}


#[derive(Debug)]
pub struct AuthProtocol {
    config: AuthConfig,
    pub connection: Connection,
}


// --------------------------------------------------------------------------------------------------------------------


impl From<EncodeError> for AuthError {
    fn from(err: EncodeError) -> Self {
        AuthError::EncodeError(err)
    }
}

impl From<ReadError> for AuthError {
    fn from(err: ReadError) -> Self {
        AuthError::ReadError(err)
    }
}

// --------------------------------------------------------------------------------------------------------------------


impl AuthProtocol {
    pub fn new(connection: Connection) -> AuthProtocol {
        AuthProtocol{
            config: AuthConfig{
                password: None,
//                password: Some("123".to_owned()),
            },
            connection: connection,
        }
    }

    fn send_message(&mut self, client_message: ClientMessage) -> Result<(), EncodeError> {
        info!("  >>  {:?}", client_message);
        let message = try!(Message::from_raw(client_message.encode()));
        Ok(self.connection.send_message(Some(message)))
    }

    pub fn auth(&mut self) -> Result<usize, AuthError> {
        try!(self.send_message(CStart::create(PROTOCOL_VERSION, 1, [].to_vec())));
        'iter_messages: loop {
            let message = try!(self.connection.read());
            match self.flow(message) {
                Workflow::Continue                  => continue 'iter_messages,
                Workflow::SwitchProtocol(client_id) => return Ok(client_id),
                Workflow::Terminate(err)            => {
                    info!("Terminated: {}", err);
                    return Err(AuthError::WorkflowError(err));
                },
            }
        }
    }
}


impl <'a> Protocol for AuthProtocol {
    #[cfg_attr(feature = "dev", trace)]
    fn flow(&self, raw_message: RawMessage) -> Workflow {
        match ServerMessage::parse(raw_message) {
            Err(err) => Workflow::Terminate(WorkflowError::Exception(format!("{:?}", err))),
            Ok(message) => {
                info!("  <<  {:?}", message);
                match message {
                    ServerMessage::Error(m) => {
                        Workflow::Terminate(WorkflowError::ProtocolError(
                            format!("Server error with message: {}", m.message)))
                    },
                    ServerMessage::Reject(m) => {
                        Workflow::Terminate(WorkflowError::ProtocolError(
                            format!("Server rejected connection with message: {}", m.reason)))
                    },
                    ServerMessage::AuthOk(m) => {
                        Workflow::SwitchProtocol(m.id)
                    },
                }
            }
        }
    }
}
