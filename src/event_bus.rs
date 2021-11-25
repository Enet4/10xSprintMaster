//! Module for the main application state bus.
//!
//! It plays a key role in applying changes to game state
//! based on user input and the game reactor.
//!

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use yew::worker::{Agent, AgentLink, Context, HandlerId};

use crate::{
    components::{stage::StageId, task::GameTaskBuilder},
    data_transfer::payload::TaskTransfer,
};

/// All messages that can be sent through the event bus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EventBusRequest {
    /// Move a task into a different stage
    MoveTask { task: TaskTransfer, to: StageId },
    /// Assign the task to a human
    AssignTask { task: TaskTransfer, human_id: u32 },
    /// Signal that a task is being dragged
    DragTaskStart(u32),
    /// Signal that the task stopped being dragged
    DragTaskEnd(u32),
    /// Add the given task details to the board.
    /// Usually employed by tutorial.
    AddTask(GameTaskBuilder),
    /// Advance the tutorial to the next phase
    AdvanceTutorial,
    /// An in-game time tick occurred.
    Tick,
    /// Advance the the next month,
    /// usually as a consequence of pressing OK on the End of Month modal.
    NextMonth,
}

pub type Request = EventBusRequest;
pub type Response = EventBusRequest;

pub struct EventBus {
    link: AgentLink<Self>,
    subscribers: HashSet<HandlerId>,
}

impl Agent for EventBus {
    type Reach = Context<Self>;
    type Message = ();
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        // just send to subscribers directly

        // if there is only one subscriber, send without cloning
        if self.subscribers.len() == 1 {
            self.link
                .respond(*self.subscribers.iter().next().unwrap(), msg);
        } else {
            for sub in self.subscribers.iter() {
                self.link.respond(*sub, msg.clone());
            }
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}
