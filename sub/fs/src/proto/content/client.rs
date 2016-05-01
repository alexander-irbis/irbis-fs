
use std::collections::hash_map::{HashMap, Entry};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use protocol::message::{Message, RawMessage};
use protocol::workflow::{Protocol, Workflow, WorkflowError};

use ::connection::StreamSender;
use ::client::connection::{Connection};
use ::types::{TaskId};

use super::message::{ClientMessage, ServerMessage};
use super::state;
use super::task::{StateHandle, StateHolder, State};


// --------------------------------------------------------------------------------------------------------------------


pub fn send_message(sender: &StreamSender, message: ClientMessage) -> Result<(), ()> {
    info!("  >>  {:?}", message);
    let message = match Message::from_raw(message.encode()) {
        Err(_)  => return Err(()),
        Ok(r)   => r
    };
    trace!("  >>>>  {:?}", message);
    ::connection::send_message(sender, Some(message));
    Ok(())
}


// --------------------------------------------------------------------------------------------------------------------


#[derive(Debug)]
pub enum RequestError {
    Error,
}


//type TaskMap<S: State> = Arc<Mutex<RefCell<HashMap<TaskId, StateHolder<S>>>>>;
type TaskMap<S> = Arc<Mutex<HashMap<TaskId, StateHolder<S>>>>;

type ContentTasks = TaskMap<state::ContentState>;


#[derive(Debug)]
pub struct ContentProtocol {
    pub connection: Connection,
    pub client_id: usize,
    _next_id: Arc<Mutex<TaskId>>,
    tasks: ContentTasks,
}


pub struct TaskInterface {
    protocol: Arc<ContentProtocol>,
    task_id: TaskId,
}


pub struct ContentInterface {
    protocol: Arc<ContentProtocol>,
}


// --------------------------------------------------------------------------------------------------------------------


impl TaskInterface {
    pub fn new(protocol: Arc<ContentProtocol>, task_id: TaskId) -> Self {
        TaskInterface { protocol: protocol, task_id: task_id }
    }

    pub fn wait(self) -> Result<state::ContentState, String> {
        self.protocol.wait(self.task_id);
        self.finish()
    }

    fn finish(self) -> Result<state::ContentState, String> {
        let state_holder: StateHolder<state::ContentState> = try!(self.protocol.finish_task(self.task_id));
        let state_handle = match Arc::try_unwrap(state_holder) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(_) => panic!("Can't unwrap Arc!"),
        };
        Ok(state_handle.task.into_inner())
    }
}


impl ContentInterface {
    pub fn new(protocol: Arc<ContentProtocol>) -> Self {
        ContentInterface { protocol: protocol }
    }

    pub fn info(&self) -> Result<TaskInterface, RequestError> {
        match self.protocol.start_task(state::GetInfo::create()) {
            Ok(task_id) => Ok(TaskInterface::new(self.protocol.clone(), task_id)),
            Err(err) => Err(err),
        }
    }

    pub fn copy_from(&self, path: &str) -> Result<TaskInterface, RequestError> {
        match self.protocol.start_task(state::CopyFrom::create(path.to_owned())) {
            Ok(task_id) => Ok(TaskInterface::new(self.protocol.clone(), task_id)),
            Err(err) => Err(err),
        }
    }
}


// --------------------------------------------------------------------------------------------------------------------


impl ContentProtocol {
    pub fn new(connection: Connection, client_id: usize) -> ContentProtocol {
        ContentProtocol {
            connection: connection,
            client_id: client_id,
            _next_id: Arc::new(Mutex::new(0)),
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }


//    /// Creates a thread that writes into the server stream each message received
//    fn create_reader_thread(&self) {
//
//        let mut stream = self.connection.stream.try_clone().unwrap();
//        let tasks = self.tasks.clone();
//
//        thread::spawn(move || {
//            'read: loop {
//                let message = match Connection::_read(&mut stream) {
//                    Ok(message) => message,
//                    Err(_) => break 'read,
//                };
//                match Self::_flow(tasks.clone(), message) {
//                    Workflow::SwitchProtocol(client_id) => {
//                        warn!("Irnored unexpected protocol switching");
//                    },
//                    Workflow::Continue                  => (),
//                    Workflow::Terminate(err)            => {
//                        info!("Terminated: {}", err);
//                        break 'read;
//                    }
//                }
//            }
//        });
//    }


    pub fn send_message(&self, message: ClientMessage) -> Result<(), ()> {
        send_message(&self.connection.sender(), message)
    }

    fn next_id(&self) -> TaskId {
        let mut next_id = self._next_id.lock().unwrap();
        *next_id += 1;
        *next_id - 1
    }

    fn start_task(&self, task: state::ContentState) -> Result<TaskId, RequestError> {
        let task_id = self.next_id();
        let state_handle = StateHandle::new(task_id, self.connection.sender(), task);
        let state_holder = Arc::new(Mutex::new(state_handle));
        {
            let mut tasks = self.tasks.lock().unwrap();
            match tasks.entry(task_id) {
//                Entry::Occupied(_) => return Err(RequestError::Error(format!("Busy task id {}", task_id))),
                Entry::Occupied(_) => return Err(RequestError::Error),
                Entry::Vacant(entry) => entry.insert(state_holder.clone()),
            };
        }

        let state_lock = state_holder.lock().unwrap();

        let mut task = state_lock.task.borrow_mut();
        task.start(&state_lock);

        Ok(task_id)
    }

    pub fn finish_task(&self, task_id: TaskId) -> Result<StateHolder<state::ContentState>, String> {

        match self.tasks.lock().unwrap().entry(task_id) {
            Entry::Occupied(entry) => Ok(entry.remove()),
            Entry::Vacant(_) => Err(format!("Absent task id {}", task_id)),
        }
    }

    pub fn wait(&self, task_id: TaskId) -> Result<(), WorkflowError> {
        let state_holder = {
            match self.tasks.lock()?.entry(task_id) {
                Entry::Vacant(_) => return Err(WorkflowError::Exception(format!("Absent task id {}", task_id))),
                Entry::Occupied(entry) => entry.get().clone(),
            }
        };

        loop {
            try!(self.read_one());
            {
                let state_lock = state_holder.lock().unwrap();
                let state = state_lock.state.borrow();
                if !state.is_waiting() { break; };
            }
            thread::sleep(time::Duration::from_millis(10));
        }
        Ok(())
    }

    pub fn read_one(&self) -> Result<(), WorkflowError> {
        let message = match self.connection.read() {
            Ok(message) => message,
            Err(_) => return Err(WorkflowError::ConnectionError)
        };
        match self.flow(message) {
            Workflow::Continue          => Ok(()),
            Workflow::SwitchProtocol(_) => {
                Err(WorkflowError::ProtocolError("Unexpected protocol switching".to_owned()))
            },
            Workflow::Terminate(err)    => {
                info!("Terminated: {}", err);
                Err(err)
            }
        }
    }

    fn _flow(tasks: ContentTasks, raw_message: RawMessage) -> Workflow {
        match ServerMessage::parse(raw_message) {
            Err(err) => Workflow::Terminate(WorkflowError::Exception(format!("{:?}", err))),
            Ok(message) => {
                info!("  <<  {:?}", message);
                let task_id = message.get_task_id();
                let r: Result<ServerMessage, Workflow> = match message {
                    ServerMessage::Error(m) => {
                        Err(Workflow::Terminate(WorkflowError::ProtocolError(
                            format!("Server error with message: {}", m.message))))
                    },
                    ServerMessage::Reject(m) => {
                        Err(Workflow::Terminate(WorkflowError::ProtocolError(
                            format!("Server rejected connection with message: {}", m.reason))))
                    },
                    ServerMessage::Info(m) => Ok(ServerMessage::Info(m)),
                    ServerMessage::CopyFrom(m) => Ok(ServerMessage::CopyFrom(m)),
                };
                match r {
                    Err(m) => m,
                    Ok(m) => {
                        let tasks = tasks.lock().unwrap();

                        let state_lock = match tasks.get(&task_id) {
                            Some(state_holder)  => state_holder.lock().unwrap(),
                            None                =>
                                // TODO ошибку в журнал и продолжить
                                return Workflow::Terminate(WorkflowError::ProtocolError(
                                    format!("Message for an absent task #{}: {:?}", task_id, m))),
                        };
                        let mut task = state_lock.task.borrow_mut();
                        match task.handle_message(&*state_lock, m) {
                            Ok(_)       => Workflow::Continue,
                            // TODO ошибку в журнал, очистить состояние и продолжить
                            Err(err)    => Workflow::Terminate(WorkflowError::ProtocolError(
                                format!("Client error while processing message: {}", err)))
                        }
                    }
                }
            }
        }
    }
}


impl Protocol for ContentProtocol {
    fn flow(&self, raw_message: RawMessage) -> Workflow {
        let tasks = self.tasks.clone();
        Self::_flow(tasks, raw_message)
    }
}


impl Drop for ContentProtocol {
    fn drop(&mut self) {
        self.connection.stream.shutdown();
    }
}

