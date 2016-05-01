

use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver};
use std::thread;


use ::connection::{StreamSender};
use ::types::{TaskId};

use super::message::{ClientMessage, ServerMessage};


// --------------------------------------------------------------------------------------------------------------------


#[derive(Debug)]
pub struct TaskHandle {
    pub task_id: TaskId,
    pub stream_tx: StreamSender,
    pub finished: Cell<bool>,
}


// --------------------------------------------------------------------------------------------------------------------


#[derive(Debug, Clone, Copy)]
pub enum SimpleState{
    Waiting,
    Error,
    Ready,
}


pub type StateHolder<T> = Arc<Mutex<StateHandle<T>>>;


#[derive(Debug)]
pub struct StateHandle<S: State> {
    pub task_id: TaskId,
    pub stream_tx: StreamSender,
    pub state: RefCell<SimpleState>,
    pub task: RefCell<S>,
}


pub trait State {
    fn start<T: State>(&mut self, task_state: &StateHandle<T>);
    fn handle_message<T: State>(&mut self, task_state: &StateHandle<T>, server_message: ServerMessage) -> Result<(), String>;
}


// --------------------------------------------------------------------------------------------------------------------


impl TaskHandle {
    pub fn new(task_id: TaskId, stream_tx: StreamSender) -> Self {
        TaskHandle {
            task_id: task_id,
            stream_tx: stream_tx,
            finished: Cell::new(false),
        }
    }
}


// --------------------------------------------------------------------------------------------------------------------


impl <S: State> StateHandle<S> {
    pub fn new(task_id: TaskId, stream_tx: StreamSender, task: S) -> StateHandle<S> {
        StateHandle {
            task_id: task_id,
            stream_tx: stream_tx,
            state: RefCell::new(SimpleState::Waiting),
            task: RefCell::new(task),
        }
    }
}


impl SimpleState {
    pub fn is_waiting(&self) -> bool {
        match *self {
            SimpleState::Waiting => true,
            _ => false,
        }
    }
    pub fn is_error(&self) -> bool {
        match *self {
            SimpleState::Error => true,
            _ => false,
        }
    }
    pub fn is_ready(&self) -> bool {
        match *self {
            SimpleState::Ready => true,
            _ => false,
        }
    }
}


