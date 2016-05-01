
use super::message;
use super::message::ServerMessage;
use super::task::{StateHandle, State, SimpleState};


#[derive(Debug)]
pub enum ContentState {
    GetInfo(GetInfo),
    CopyFrom(CopyFrom),
}


#[derive(Debug)]
pub struct GetInfo {
    pub response: Option<message::SInfo>,
}


#[derive(Debug)]
pub struct CopyFrom {
    uri: String,
    pub result: Option<message::SCopyFrom>,
}


// --------------------------------------------------------------------------------------------------------------------


impl State for ContentState {
    fn start<T: State>(&mut self, task_state: &StateHandle<T>) {
        match *self {
            ContentState::GetInfo(ref mut s) => s.start(task_state),
            ContentState::CopyFrom(ref mut s) => s.start(task_state),
        }
    }
    fn handle_message<T: State>(&mut self, task_state: &StateHandle<T>, message: ServerMessage) -> Result<(), String> {
        match *self {
            ContentState::GetInfo(ref mut s) => s.handle_message(task_state, message),
            ContentState::CopyFrom(ref mut s) => s.handle_message(task_state, message),
        }
    }
}


// --------------------------------------------------------------------------------------------------------------------

impl <'a> GetInfo {
    pub fn create() -> ContentState {
        ContentState::GetInfo(GetInfo { response: None })
    }
}


impl State for GetInfo {
    fn start<T: State>(&mut self, task_state: &StateHandle<T>) {
        super::client::send_message(&task_state.stream_tx, message::CGetInfo::create(task_state.task_id));
    }

    fn handle_message<T: State>(&mut self, task_state: &StateHandle<T>, server_message: ServerMessage) -> Result<(), String> {
        match server_message {
            ServerMessage::Info(info) => {
                self.response = Some(info);
                *task_state.state.borrow_mut() = SimpleState::Ready;
                Ok(())
            },
            _ => {
                *task_state.state.borrow_mut() = SimpleState::Error;
                Err(format!("Unexpected message {:?} for task {:?}", server_message, self))
            }
        }
    }
}


impl <'a> CopyFrom {
    pub fn create(uri: String) -> ContentState {
        ContentState::CopyFrom(CopyFrom { uri: uri, result: None })
    }
}


impl State for CopyFrom {
    fn start<T: State>(&mut self, task_state: &StateHandle<T>) {
        super::client::send_message(&task_state.stream_tx, message::CCopyFrom::create(task_state.task_id, self.uri.clone()));
    }

    fn handle_message<T: State>(&mut self, task_state: &StateHandle<T>, server_message: ServerMessage) -> Result<(), String> {
        match server_message {
            ServerMessage::CopyFrom(copy_from) => {
                self.result = Some(copy_from);
                *task_state.state.borrow_mut() = SimpleState::Ready;
                Ok(())
            },
            _ => {
                *task_state.state.borrow_mut() = SimpleState::Error;
                Err(format!("Unexpected message {:?} for task {:?}", server_message, self))
            }
        }
    }
}
