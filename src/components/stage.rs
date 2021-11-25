use serde::{Deserialize, Serialize};
use yew::{agent::Dispatcher, prelude::*};

use crate::{
    data_transfer::{payload::TaskTransfer, DataTransfer, DragEffect},
    event_bus::{EventBus, EventBusRequest},
};

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    /// The unique identifier for the stage
    pub id: StageId,
    /// A human readable description of the stage
    pub description: String,
    /*
    #[prop_or_default]
    pub children: ChildrenWithProps<super::task::Task>,
    */
    // note: could not yet use ChildrenWithProps
    // because I could not get it to work with the view method in `Game`
    #[prop_or_default]
    pub children: Children,
}

/// View component for a stage (column) of the workboard.
pub struct Stage {
    props: Props,
    link: ComponentLink<Self>,
    event_bus: Dispatcher<EventBus>,
}

/// The unique identifier for a stage (column) in the workboard.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum StageId {
    /// Backlog, ideas, fresh and underspecified tasks.
    #[serde(rename = "backlog")]
    Backlog,
    /// Spring candidate, for tasks to consider pursuing.
    /// Tasks must be fully specified before moving on from here.
    #[serde(rename = "candidate")]
    Candidate,
    /// Under development progress.
    #[serde(rename = "progress")]
    Progress,
    /// Under peer review.
    #[serde(rename = "review")]
    Review,
    /// For tasks which are complete.
    #[serde(rename = "done")]
    Done,
}

impl StageId {
    /// Convert to
    pub fn to_str(self) -> &'static str {
        match self {
            StageId::Backlog => "backlog",
            StageId::Candidate => "candidate",
            StageId::Progress => "progress",
            StageId::Review => "review",
            StageId::Done => "done",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    /// For when there is still a dragged task moving on the stage.
    TaskOver,
    /// For when a dragged task entered the stage.
    CheckTask(TaskTransfer),
    /// For when a dragged task was dropped onto the stage.
    DropTask(TaskTransfer),
}

impl Component for Stage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Stage {
            props,
            link,
            event_bus: EventBus::dispatcher(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::TaskOver => false,
            Msg::CheckTask(_task_transfer) => false,
            Msg::DropTask(task_transfer) => {
                // emit request to dispatcher
                if task_transfer.from != self.props.id {
                    self.event_bus.send(EventBusRequest::MoveTask {
                        task: task_transfer,
                        to: self.props.id,
                    });
                    true
                } else {
                    false
                }
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
        let id = self.props.id.clone();
        let dragenter_handler = self.link.callback(move |ev: DragEvent| {
            {
                let ev: &Event = ev.as_ref();
                ev.prevent_default();
            }
            Msg::TaskOver
        });
        let dragover_handler = self.link.callback(|ev: DragEvent| {
            {
                let ev: &Event = ev.as_ref();
                ev.prevent_default();
            }
            let data_transfer = DataTransfer::from_event(&ev);
            data_transfer.set_drop_effect(DragEffect::Move);
            Msg::TaskOver
        });
        let drop_handler = self.link.callback(move |ev: DragEvent| {
            gloo_console::debug!("DROP", &ev);
            {
                let ev: &Event = ev.as_ref();
                ev.prevent_default();
            }
            // get data transfer item
            // (so that this drop zone knows what task was dropped)
            let data_transfer = DataTransfer::from_event(&ev);

            let data: TaskTransfer = data_transfer.get_data();
            gloo_console::log!(
                "Dropped task",
                data.id,
                "from",
                data.from.to_str(),
                "to",
                id.to_str()
            );
            Msg::DropTask(data)
        });

        let e_id = format!("stage_{}", self.props.id.to_str());
        html! {
            <div id=e_id
                    class=classes!("board-stage", {format!("board-stage-{}", self.props.id.to_str())})
                    ondrop=drop_handler ondragover=dragover_handler ondragenter=dragenter_handler>
                <div class="board-stage-header">{&self.props.description}</div>
                <div class="board-stage-body">
                    {for self.props.children.iter()}
                </div>
            </div>
        }
    }
}
