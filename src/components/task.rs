use std::borrow::Cow;

use rand::Rng;
use rand_distr::{Distribution, Standard};
use serde::{Deserialize, Serialize};
use yew::{agent::Dispatcher, prelude::*};

use crate::{
    components::{bug, progress_bar},
    data_transfer::{payload::TaskTransfer, DataTransfer, DragEffect},
    event_bus::{EventBus, EventBusRequest},
};

use super::stage::StageId;

/// A full descriptor for a game task
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GameTask {
    /// the unique task ID
    pub id: u32,
    /// a description of the task
    pub description: String,
    /// the kind of task
    pub kind: TaskKind,
    /// At which stage the task currently is.
    pub stage: StageId,
    /// the ID of the user assigned to the task
    pub assigned: Option<u32>,
    /// the ID of the user who completed the development of the task
    pub developed_by: Option<u32>,
    /// The task's score to be awarded when complete.
    ///
    /// In some kinds of tasks, this is always zero.
    pub score: i32,
    /// A measurement of difficulty or effort to complete the task.
    /// Should be around 10 on average.
    pub difficulty: u32,
    /// The progress of the task, from 0 to 1.
    /// In sprint candidates, this is the writing progress.
    /// In progress, this is the progress of development.
    pub progress: f64,
    /// Whether the task is fully specified.
    pub specified: bool,
    /// the number of bugs introduced while developing for this task
    /// and will be added to the software project if merged
    pub bugs: u32,
    /// the number of bugs found through review
    pub bugs_found: u32,
    /// Whether the task is visible
    /// (old tasks already done are hidden in the subsequent month)
    pub visible: bool,
}

/// Details for constructing a new task.
///
/// Used by world state actually create a new task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameTaskBuilder {
    /// a description of the task
    pub description: String,
    /// the kind of task
    pub kind: TaskKind,
    /// The task's score to be awarded when complete.
    ///
    /// In some kinds of tasks, this is always zero.
    pub score: i32,
    /// A measurement of difficulty or effort to complete the task.
    /// Should be around 10 on average.
    pub difficulty: u32,
}

impl GameTaskBuilder {
    /// Create a new task builder
    pub fn new(
        description: impl Into<String>,
        kind: TaskKind,
        score: i32,
        difficulty: u32,
    ) -> Self {
        Self {
            description: description.into(),
            kind,
            score,
            difficulty,
        }
    }
}

impl GameTask {
    /// Create a new task in the backlog
    pub fn new(
        id: u32,
        description: impl Into<String>,
        kind: TaskKind,
        score: i32,
        difficulty: u32,
    ) -> Self {
        Self {
            id,
            description: description.into(),
            kind,
            stage: StageId::Backlog,
            assigned: None,
            developed_by: None,
            score,
            difficulty,
            progress: 0.,
            specified: false,
            bugs: if kind == TaskKind::Bug { 1 } else { 0 },
            bugs_found: if kind == TaskKind::Bug { 1 } else { 0 },
            visible: true,
        }
    }

    /// Add some progress.
    /// Return whether the task has reached 100% progress.
    pub fn add_progress(&mut self, progress: f64) -> bool {
        self.progress = (self.progress + progress).min(1.);

        if self.progress >= 1. {
            if self.stage == StageId::Candidate {
                self.specified = true;
            }
            true
        } else {
            false
        }
    }

    pub fn reset_progress(&mut self) {
        self.progress = 0.;
    }

    pub fn assign(&mut self, user_id: u32) {
        self.assigned = Some(user_id);
    }

    /// Moves the task to a new stage
    /// and updates its progress accordingly.
    pub fn assign_to_stage(&mut self, stage: StageId) {
        match (self.stage, stage) {
            (StageId::Backlog, StageId::Candidate) => {
                // progress will refer to writing progress
                self.progress = 0.;
            }
            (StageId::Candidate, StageId::Progress) => {
                // progress will now refer to development progress
                self.progress = 0.;
            }
            _ => {}
        }
        self.stage = stage;
    }

    /// Whether the requirements have already been fully specified.
    pub fn is_specified(&self) -> bool {
        self.specified
    }

    /// Whether the task has been fully developed
    pub fn is_developed(&self) -> bool {
        match self.stage {
            StageId::Backlog | StageId::Candidate => false,
            StageId::Progress | StageId::Review => self.progress >= 1.,
            StageId::Done => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    /// the unique task ID
    pub id: u32,
    /// a description of the task
    #[prop_or_default]
    pub description: String,
    /// the kind of task
    #[prop_or_default]
    pub kind: TaskKind,
    /// At which stage the task currently is.
    ///
    /// Note that this should be made consistent with its position in the DOM,
    /// no checks are made to ensure this at the moment!
    pub stage: StageId,
    /// the user assigned to the task (ID and name)
    #[prop_or_default]
    pub assigned: Option<(u32, String, String)>,
    /// The task's score to be awarded when complete.
    ///
    /// In some kinds of tasks, this is always zero.
    #[prop_or(0)]
    pub score: i32,
    /// The progress of the task, from 0 to 1.
    /// In sprint candidates, this is the writing progress.
    /// In progress, this is the progress of development.
    #[prop_or(0.)]
    pub progress: f64,
    /// The number of bugs in the task found through review
    #[prop_or(0)]
    pub bugs_found: u32,
}

/// View component for a task (done or to be done).
///
/// A task goes through multiple stages:
///
/// - it is first created in the _backlog_
///   in response to certain events or research time
/// - Then, it must be moved to _sprint candidate_
///   for requirement lifting.
///   This takes some writing time (progress).
/// - Once done, the task must be assigned to a developer.
pub struct Task {
    props: Props,
    link: ComponentLink<Self>,
    event_bus: Dispatcher<EventBus>,
}

/// An enumeration for the kind of task
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskKind {
    /// normal tasks, bring score rewards when done
    #[serde(rename = "normal")]
    Normal,
    /// bug tasks, reduce score lingering and bring small rewards when done
    #[serde(rename = "bug")]
    Bug,
    /// chore tasks, decrease technical debt when done
    #[serde(rename = "chore")]
    Chore,
}

impl Default for TaskKind {
    fn default() -> Self {
        TaskKind::Normal
    }
}

impl Distribution<TaskKind> for Standard {
    fn sample<R: ?Sized>(&self, rng: &mut R) -> TaskKind
    where
        R: Rng,
    {
        // normal: 4 / 8
        // bug: 1 / 8
        // chore: 3 / 8

        let index: u8 = rng.gen_range(0..8);
        match index {
            0..=3 => TaskKind::Normal,
            4 => TaskKind::Bug,
            5..=7 => TaskKind::Chore,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum Msg {
    /// The user started dragging the task.
    StartDrag,
    /// The user stopped dragging the task.
    EndDrag,
    /// The user is dragging the task.
    Drag,
    /// The cursor is over the task.
    Hover,
}

impl Component for Task {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Task {
            props,
            link,
            event_bus: EventBus::dispatcher(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::StartDrag => {
                // send message to hub
                self.event_bus
                    .send(EventBusRequest::DragTaskStart(self.props.id));
            }
            Msg::EndDrag => {
                // send message to hub
                self.event_bus
                    .send(EventBusRequest::DragTaskEnd(self.props.id));
            }
            _ => {}
        }
        false
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
        let mut classes = vec![];
        let class_kind = match self.props.kind {
            TaskKind::Normal => "board-task",
            TaskKind::Chore => "board-task board-task-chore",
            TaskKind::Bug => "board-task board-task-bug",
        };
        classes.push(class_kind);
        if self.props.progress >= 1. {
            classes.push("board-task-complete");
        }
        if self.props.bugs_found > 0 {
            classes.push("board-task-with-bug");
        }

        let task_id = self.props.id;
        let task_stage = self.props.stage;
        let task_kind = self.props.kind;
        let task_progress = self.props.progress;
        let dragstart_handler = self.link.callback(move |ev: DragEvent| {
            // set data transfer item
            // (so that drops zone know what task was dropped)
            let data_transfer = DataTransfer::from_event(&ev);
            gloo_console::debug!("Moving task", task_id);

            let content = TaskTransfer {
                id: task_id,
                kind: task_kind,
                from: task_stage,
                progress: task_progress,
            };

            // set "move" as the drag effect
            data_transfer.set_data(&content);
            data_transfer.set_drop_effect(DragEffect::Move);

            Msg::StartDrag
        });

        let dragend_handler = self.link.callback(move |_ev: DragEvent| {
            Msg::EndDrag
        });

        let drag_handler = self.link.callback(|ev: DragEvent| {
            let ev: &Event = ev.as_ref();
            ev.prevent_default();
            Msg::Drag
        });

        let mouseenter_handler = self.link.callback(|ev: MouseEvent| {
            let ev: &Event = ev.as_ref();
            ev.prevent_default();
            Msg::Hover
        });
        let mouseleave_handler = self.link.callback(|ev: MouseEvent| {
            let ev: &Event = ev.as_ref();
            ev.prevent_default();
            Msg::Hover
        });

        let e_id = format!("task_T{}", self.props.id);
        let description = Some(Cow::Owned(self.props.description.clone()));

        let task_score_class = "board-task-score";
        html! {
            <div id=e_id class=classes draggable="true" title=description
                 ondrag=drag_handler
                 ondragstart=dragstart_handler
                 ondragend=dragend_handler
                 onmouseenter=mouseenter_handler
                 onmouseleave=mouseleave_handler>
                {
                    if self.props.bugs_found > 0 {
                        bug()
                    } else {
                        html! {}
                    }
                }
                {"T"}{self.props.id}
                {
                    if let Some((_user_id, user_name, color_code)) = &self.props.assigned {
                        let tooltip = format!("Assigned to {}", user_name);
                        let style = format!("background-color: {}", color_code);
                        html! {
                            <div class="board-task-assigned" title=tooltip.clone()>
                                <div class="board-task-assigned-inner" style=style />
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
                <span class=task_score_class>{"+"} {self.props.score}</span>

                {
                    if self.props.progress > 0. && self.props.stage != StageId::Done {
                        progress_bar("board-task-progress-outer", "board-task-progress-inner", self.props.progress as f32)
                    } else {
                        html! {}
                    }
                }
            </div>
        }
    }
}
