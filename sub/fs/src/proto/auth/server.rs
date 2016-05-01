
use std::cell::Cell;

use protocol::message::{Message, RawMessage};
use protocol::workflow::{Protocol, Workflow, WorkflowError};

use ::connection::{StreamSender};
use super::message::{ClientMessage, ServerMessage, SAuthOk};

// --------------------------------------------------------------------------------------------------------------------


#[derive(Debug)]
pub struct AuthConfig {
    need_password: bool
}


#[derive(Debug, Clone, Copy)]
pub enum AuthProtocolStage {
    BeforeStart,
    NeedAuth,
    Ok
}


#[derive(Debug)]
pub struct AuthProtocol {
    config: AuthConfig,
    stage: Cell<AuthProtocolStage>,
    pub id: usize,
    pub sender: StreamSender,
}


// --------------------------------------------------------------------------------------------------------------------


pub fn send_message(sender: &StreamSender, message: ServerMessage) -> Result<(), ()> {
    info!("  <<  {:?}", message);
    let message = match Message::from_raw(message.encode()) {
        Err(_)  => return Err(()),
        Ok(r)   => r
    };
    trace!("  <<<<  {:?}", message);
    ::connection::send_message(sender, Some(message));
    Ok(())
}


// --------------------------------------------------------------------------------------------------------------------


impl AuthProtocol {
    pub fn new(sender: StreamSender, id: usize) -> AuthProtocol {
        AuthProtocol{
            config: AuthConfig{
                need_password: false,
            },
            stage: Cell::new(AuthProtocolStage::BeforeStart),
            id: id,
            sender: sender,
        }
    }

    pub fn send_message(&self, message: ServerMessage) -> Result<(), ()> {
        send_message(&self.sender, message)
    }
}


impl <'a> Protocol for AuthProtocol {
    fn flow(&self, raw_message: RawMessage) -> Workflow {
        match ClientMessage::parse(raw_message) {
            Err(err) => Workflow::Terminate(WorkflowError::Exception(format!("{:?}", err))),
            Ok(v) => {
                info!("  >>  {:?}", v);
                match v {
                    ClientMessage::Start(c) => match self.stage.get() {
                        AuthProtocolStage::BeforeStart => {
                            if c.subprotocol != 1 {
                                let error = format!("Wrong requested protocol: {:?}", c.subprotocol);
                                error!("Wrong requested protocol: {:?}", c.subprotocol);
                                return Workflow::Terminate(WorkflowError::Exception(error))
                            }

                            // TODO аутентификация клиента
                            // self.stage = NeedAuth;
                            //Workflow::Continue

                            self.stage.set(AuthProtocolStage::Ok);
                            match self.send_message(SAuthOk::create(0)) {
                                Ok(_)   => {
                                    Workflow::SwitchProtocol(0)
                                },
                                Err(_)  => Workflow::Terminate(WorkflowError::ConnectionError)
                            }
                        },
                        _ => Workflow::Terminate(WorkflowError::Exception("Wrong message order".to_owned()))
                    },
                }
            }
        }
    }
}
