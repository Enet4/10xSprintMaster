use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};
use yew::{agent::Dispatcher, prelude::*};

use crate::{
    data_transfer::{payload::TaskTransfer, DataTransfer, DragEffect},
    event_bus::{EventBus, EventBusRequest},
};

/// A human resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameHuman {
    /// unique ID
    pub id: u32,
    /// name
    pub name: Cow<'static, str>,
    /// The human's color code
    pub color: Cow<'static, str>,
    /// current status
    pub status: HumanStatus,
    /// the experience level of the human
    pub experience: u32,
    /// the progress of the human at doing something
    pub progress: f32,
    /// whether the human quit the team and is not coming back
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub quit: bool,
}

fn is_false(x: &bool) -> bool {
    !x
}

impl GameHuman {
    /// Create a new human.
    pub fn new(
        id: u32,
        name: impl Into<Cow<'static, str>>,
        color_code: impl Into<Cow<'static, str>>,
        initial_experience: u32,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            color: color_code.into(),
            status: HumanStatus::Idle,
            experience: initial_experience,
            progress: 0.,
            quit: false,
        }
    }
}

/// Props for the Human component.
#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    /// Unique ID
    pub id: u32,
    /// The name of the human
    pub name: Cow<'static, str>,
    /// The human's color code
    pub color: Cow<'static, str>,
    /// The human's current status
    pub status: HumanStatus,
    /// whether to bring humans upwards for assignment
    pub bring_up: bool,
}

/// A status of the human.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum HumanStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "writing")]
    Writing,
    #[serde(rename = "coding")]
    Coding,
    #[serde(rename = "reviewing")]
    Reviewing,
}

impl Default for HumanStatus {
    fn default() -> Self {
        HumanStatus::Idle
    }
}

impl Display for HumanStatus {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            HumanStatus::Idle => write!(f, "idle"),
            HumanStatus::Writing => write!(f, "writing"),
            HumanStatus::Coding => write!(f, "coding"),
            HumanStatus::Reviewing => write!(f, "reviewing"),
        }
    }
}

pub struct Human {
    props: Props,
    link: ComponentLink<Self>,
    event_bus: Dispatcher<EventBus>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Nothing,
    Assign(TaskTransfer),
}

/// The data transferred when dragging a human to a drop zone.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct HumanTransfer {
    /// the unique user ID (0 is You)
    pub id: u32,
    pub name: String,
}

impl Component for Human {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Human {
            props,
            link,
            event_bus: EventBus::dispatcher(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Nothing => {
                // do nothing
                false
            }
            Msg::Assign(task) => {
                // emit request to dispatcher
                self.event_bus.send(EventBusRequest::AssignTask {
                    task,
                    human_id: self.props.id,
                });
                true
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.props = props;
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        let name = self.props.name.clone();

        let dragenter_handler = self.link.callback(move |ev: DragEvent| {
            {
                let ev: &Event = ev.as_ref();
                ev.prevent_default();
            }
            Msg::Nothing
        });
        let dragover_handler = self.link.callback(|ev: DragEvent| {
            {
                let ev: &Event = ev.as_ref();
                ev.prevent_default();
            }
            let data_transfer = DataTransfer::from_event(&ev);
            data_transfer.set_drop_effect(DragEffect::Link);
            Msg::Nothing
        });
        let drop_handler = self.link.callback(move |ev: DragEvent| {
            {
                let ev: &Event = ev.as_ref();
                ev.prevent_default();
            }
            // get data transfer item
            // (so that this drop zone knows what task was dropped)
            let data_transfer = DataTransfer::from_event(&ev);
            let data: TaskTransfer = data_transfer.get_data();

            // assign task to human
            Msg::Assign(data)
        });

        let status_class = format!("human-status-{}", self.props.status);

        let name_style = format!("border-color: {}", self.props.color);

        let outer_classes = match (self.props.bring_up, self.props.id == 0) {
            (false, false) => "human-outer",
            (true, false) => "human-outer human-outer-up",
            (false, true) => "human-outer you",
            (true, true) => "human-outer human-outer-up you",
        };

        let status = match self.props.status {
            HumanStatus::Idle => "",
            HumanStatus::Writing => "✍",
            HumanStatus::Coding => "⌨",
            HumanStatus::Reviewing => "👀",
        };

        // humans are drop zones: drop a task on it to assign it to this human
        html! {
            <div class=outer_classes
                    ondragenter=dragenter_handler
                    ondragover=dragover_handler
                    ondrop=drop_handler>
                <div class=classes!("human-activity", status_class)>{status}</div>
                <div class="human-head">
                    <div class="human-eye">
                      <div class="human-eye-pupil" />
                    </div>
                    <div class="human-eye">
                        <div class="human-eye-pupil" />
                    </div>
                </div>
                <div class="human-body"></div>
                <div class="human-name" style=name_style>{ name }</div>
            </div>
        }
    }
}
