
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::thread;

use compat::{getpid, getos};
use ::server::database::{Database, DatabaseHolder};

use super::message::*;
use super::task::{TaskHandle};
use super::server::{send_message};

#[cfg(all(target_pointer_width = "32"))] const BITS: u16 = 32;
#[cfg(all(target_pointer_width = "64"))] const BITS: u16 = 64;


#[derive(Debug)]
pub enum ContentAction {
    GetInfo,
    CopyFrom(CopyFrom),
}


#[derive(Debug)]
pub struct GetInfo;

#[derive(Debug)]
pub struct CopyFrom {
    pub uri: String,
    pub db: DatabaseHolder,
}


// --------------------------------------------------------------------------------------------------------------------


pub type TaskHolder = Arc<Mutex<TaskContainer>>;

#[derive(Debug)]
pub struct TaskContainer {
    pub handle: TaskHandle,
    pub action: ContentAction,
}


impl ContentAction {
    pub fn start(&self, task: TaskHolder, rx: Receiver<ClientMessage>) -> thread::JoinHandle<()> {
        match *self {
            ContentAction::GetInfo => get_info(task, rx),
            ContentAction::CopyFrom(ref a) => copy_from(task, rx),
        }
    }
}


// --------------------------------------------------------------------------------------------------------------------

impl TaskContainer {
    pub fn new(handle: TaskHandle, action: ContentAction) -> Self {
        TaskContainer {
            handle: handle,
            action: action,
        }
    }
}

fn get_info(task: TaskHolder, rx: Receiver<ClientMessage>) -> thread::JoinHandle<()> {
    use super::message::{SInfo};

    thread::spawn(move || {
        let os = getos();
        let task = task.lock().unwrap();
        let info = SInfo::create(
            task.handle.task_id,
            getpid(), // FIXME pid сервера
            BITS,
            format!("{} {} {}", os.0, os.1, os.2),
        );
        send_message(&task.handle.stream_tx, info);
        task.handle.finished.set(true);
    })
}


fn copy_from(task: TaskHolder, rx: Receiver<ClientMessage>) -> thread::JoinHandle<()> {
    use super::message::{SError, SCopyFrom, SCopyFromState};

    thread::spawn(move || {
        let (db, uri) = match task.lock().unwrap().action {
            ContentAction::CopyFrom(ref action) => (action.db.clone(), action.uri.clone()),
            _ => unreachable!(),
        };
        let result = Database::copy_from(db, &uri);
        {
            let mut task = task.lock().unwrap();
            match result {
                Ok(result) => {
                    let result = SCopyFromState::Complete(result);
                    send_message(&task.handle.stream_tx, SCopyFrom::create(task.handle.task_id, result));
                },
                Err(err) => {
                    send_message(&task.handle.stream_tx, SError::create(task.handle.task_id, format!("{:?}", err)));
                },
            };
            task.handle.finished.set(true);
        }
    })
}
