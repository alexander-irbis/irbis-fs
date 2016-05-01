
use std::collections::hash_map::{HashMap, Entry};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};

use protocol::message::{Message, RawMessage};
use protocol::workflow::{Protocol, Workflow, WorkflowError};

use ::connection::StreamSender;
use ::server::database::DatabaseHolder;
use ::types::{TaskId};

use super::actions;
use super::actions::{ContentAction, TaskHolder, TaskContainer};
use super::message::{ClientMessage, ServerMessage};
use super::task::{TaskHandle};


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


#[derive(Debug)]
pub struct ContentConfig {
    pub root: String,
}


type TaskMap<T> = Arc<Mutex<HashMap<TaskId, T>>>;


#[derive(Debug)]
pub struct ContentProtocol {
    pub db: DatabaseHolder,
    pub id: usize,
    pub sender: StreamSender,
    tasks_h: TaskMap<TaskHolder>,
    tasks_tx: TaskMap<Sender<ClientMessage>>,
}



// --------------------------------------------------------------------------------------------------------------------


impl ContentProtocol {
    pub fn new(sender: StreamSender, id: usize, db: DatabaseHolder) -> ContentProtocol {
        ContentProtocol {
            db: db,
            id: id,
            sender: sender,
            tasks_h: Arc::new(Mutex::new(HashMap::new())),
            tasks_tx: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start_task(&self, task_id: TaskId, action: actions::ContentAction) -> Result<(), String> {
        let (tx, rx) = channel::<ClientMessage>();
        let handle = TaskHandle::new(task_id, self.sender.clone());
        let container = TaskContainer::new(handle, action);
        let task_holder = Arc::new(Mutex::new(container));
        {
            let mut tasks_h = self.tasks_h.lock().unwrap();
            let mut tasks_tx = self.tasks_tx.lock().unwrap();
            match tasks_h.entry(task_id) {
                Entry::Occupied(_) => return Err(format!("Duplicate task id {}", task_id)),
                Entry::Vacant(entry) => {
                    tasks_tx.insert(task_id, tx);
                    entry.insert(task_holder.clone())
                },
            };
        }
        let container_ = task_holder.lock().unwrap();
        container_.action.start(task_holder.clone(), rx);
        Ok(())
    }

    pub fn finish_task(&self, task_id: TaskId) -> Result<(), String> {
        let mut tasks_h = self.tasks_h.lock().unwrap();
        let mut tasks_tx = self.tasks_tx.lock().unwrap();
        match tasks_h.entry(task_id) {
            Entry::Vacant(_) => return Err(format!("Absent task id {}", task_id)),
            Entry::Occupied(entry) => {
                tasks_tx.remove(&task_id);
                entry.remove()
            },
        };
        Ok(())
    }
}


//impl <'a> Drop for ContentProtocol<'a> {
//    fn drop(mut self) {
//
//    }
//}


impl Protocol for ContentProtocol {
    fn flow(&self, raw_message: RawMessage) -> Workflow {
        match ClientMessage::parse(raw_message) {
            Err(err) => Workflow::Terminate(WorkflowError::Exception(format!("{:?}", err))),
            Ok(v) => {
                info!("  >>  {:?}", v);
                let (task_id, action) = match v {
                    ClientMessage::GetInfo(m) => (m.task_id, ContentAction::GetInfo),
                    ClientMessage::CopyFrom(m) => (m.task_id,
                        ContentAction::CopyFrom(actions::CopyFrom{uri: m.uri, db: self.db.clone()})
                    ),
                };
                match self.start_task(task_id, action) {
                    Ok(_)       => Workflow::Continue,
                    Err(err)    => Workflow::Terminate(WorkflowError::ProtocolError(err))
                }
            }
        }
    }
}
