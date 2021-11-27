use gloo_timers::callback::Timeout;
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;

use crate::audio::{play_endofmonth, play_zipclick};
use crate::components::stage::StageId;
use crate::components::task::{GameTaskBuilder, TaskKind};
use crate::components::{
    board::Board, clock::Clock, human::Human, modal::Modal, stage::Stage, task::Task,
};
use crate::event_bus::{EventBus, EventBusRequest};
use crate::services::{EventReactor, GameSpeed, GameWatch};
use crate::state::{EventOutcome, WorldState};

use super::messages::Message;
use super::task::GameTask;

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    /// How the game state should be loaded.
    pub state_from: GameStateOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameStateOrigin {
    New {
        project_name: String,
        onboarding: bool,
    },
    Continue,
    Dummy,
}

#[derive(Debug)]
pub enum Msg {
    /// a request to close the modal, should resume game
    CloseModal,
    /// an event was received via the event bus
    Event(EventBusRequest),
    /// an event to be triggered after some milliseconds
    EventWithTimeout { ms: u32, event: EventBusRequest },
    /// an event to save the game in its current state to local storage
    SaveGame,
    /// user-triggererd event to pause the game
    Pause,
    /// an event to change the speed of in-game time
    SetGameSpeed(GameSpeed),
    /// enable or disable sound
    ToggleSound,
}

pub struct Game {
    props: Props,
    link: ComponentLink<Self>,

    /// Whether the modal is visible.
    /// The game is paused while so.
    modal: Option<Message>,

    /// The full game state.
    state: WorldState,

    /// Whether audio is enabled.
    sound_enabled: bool,

    /// Producer of random events.
    reactor: EventReactor,

    /// Whether to raise the humans a bit upwards
    /// so that they are easier to assign tasks to.
    bring_humans_up: bool,

    /// event dispatcher
    dispatch: Box<dyn Bridge<EventBus>>,

    /// game watch, produces ticks over a time interval
    watch: GameWatch,
}

impl Component for Game {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        // Create Dispatch with a bridge that receives new state.
        let dispatch = EventBus::bridge(link.callback(Msg::Event));

        // choose how to load the game
        let state = match &props.state_from {
            GameStateOrigin::New {
                project_name,
                onboarding,
            } => {
                let state = WorldState::new(project_name.clone(), *onboarding);
                state.save().expect_throw("could not save game");
                state
            }
            GameStateOrigin::Continue => WorldState::load_from_storage()
                .expect_throw("could not load game :(")
                .expect_throw("no save game!"),
            GameStateOrigin::Dummy => WorldState::dummy(),
        };

        let sound_enabled =
            crate::audio::is_enabled().expect_throw("Could not load audio settings");

        let mut watch = GameWatch::new();

        // start the watch
        {
            let link = link.clone();
            let tick_fn = move || link.send_message(Msg::Event(EventBusRequest::Tick));
            watch.start_with(tick_fn);
        }

        // bring tutorial in after a small while
        if state.tutorial.is_some() {
            let link = link.clone();
            let tutorial_fn =
                move || link.send_message(Msg::Event(EventBusRequest::AdvanceTutorial));
            Timeout::new(700, tutorial_fn).forget();
        }

        Self {
            link,
            props,
            modal: None,
            state,
            sound_enabled,
            bring_humans_up: false,
            reactor: EventReactor::new(),
            dispatch,
            watch,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Pause => {
                self.watch.pause();
                true
            }
            Msg::SetGameSpeed(speed) => {
                if Some(speed) != self.watch.current_speed() {
                    self.update_speed(speed);
                    gloo_console::debug!("Game speed set to", speed.to_string());
                    true
                } else {
                    false
                }
            }
            Msg::ToggleSound => {
                self.sound_enabled = !self.sound_enabled;
                crate::audio::set_audio(self.sound_enabled).expect_throw("Could not save audio settings");
                if self.sound_enabled {
                    play_zipclick();
                }
                true
            }
            Msg::CloseModal => {
                self.modal = None;
                // resume game
                let link = self.link.clone();
                let tick_fn = move || link.send_message(Msg::Event(EventBusRequest::Tick));
                self.watch.start_with(tick_fn);

                true
            }

            Msg::Event(EventBusRequest::DragTaskStart(_task_id)) => {
                self.bring_humans_up = true;
                true
            }

            Msg::Event(EventBusRequest::DragTaskEnd(_task_id)) => {
                self.bring_humans_up = false;
                true
            }

            Msg::Event(event) => {
                let updated = match event {
                    EventBusRequest::AssignTask { .. } | EventBusRequest::MoveTask { .. } => {
                        self.bring_humans_up = false;
                        true
                    }
                    _ => false,
                };

                match self.state.apply_event(event, &mut self.reactor) {
                    EventOutcome::Nothing => updated,
                    EventOutcome::Update => true,
                    EventOutcome::Alert(msg) => {
                        // TODO show alert

                        gloo_console::warn!("alert:", msg);
                        true
                    }
                    EventOutcome::OpenMessage(msg) => {
                        // apply modal
                        self.modal = Some(msg);
                        // pause the game
                        self.watch.pause();
                        true
                    }
                    EventOutcome::EndOfMonth(report) => {
                        // apply modal
                        self.modal = Some(Message::EndOfMonth(report));

                        // play end of month tune
                        play_endofmonth();

                        // pause the game
                        self.watch.pause();
                        true
                    }
                }
            }
            Msg::EventWithTimeout { ms, event } => {
                let link = self.link.clone();
                let event_fn = move || link.send_message(Msg::Event(event));
                Timeout::new(ms, event_fn).forget();

                false
            }
            Msg::SaveGame => {
                self.state.save().expect_throw("could not save game");
                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if props != self.props {
            self.props = props;
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        let modal = match &self.modal {
            Some(Message::Simple { title, message }) => {
                let click_handler = self.link.callback(|_| {
                    play_zipclick();

                    Msg::CloseModal
                });

                html! {
                    <Modal title={title.clone()}>
                        <p>
                            { message.clone() }
                        </p>
                        <button onclick=click_handler>{ "OK" }</button>
                    </Modal>
                }
            }
            Some(message @ Message::EndOfMonth(_)) => {
                let click_handler = self.link.batch_callback(|_| {
                    play_zipclick();

                    vec![
                        Msg::Event(EventBusRequest::NextMonth),
                        Msg::CloseModal,
                        Msg::SaveGame,
                    ]
                });

                let title = format!("End of Month {}", self.state.month);
                html! {
                    <Modal title=title>
                        {message.body()}
                        <button onclick=click_handler>{ "OK" }</button>
                    </Modal>
                }
            }
            Some(message @ Message::Tutorial(phase)) => {
                gloo_console::debug!("Presenting tutorial message ", *phase);

                match phase {
                    // do not produce any modals
                    0 => {
                        html! {}
                    }
                    // advance automatically to next message
                    1 | 2 => {
                        let click_handler = self.link.callback(move |_| {
                            play_zipclick();

                            Msg::Event(EventBusRequest::AdvanceTutorial)
                        });

                        html! {
                            <Modal title="Onboarding">
                                { message.body() }
                                <button onclick=click_handler>{ "OK" }</button>
                            </Modal>
                        }
                    }
                    // perform timed stuff, then advance
                    3 => {
                        let click_handler = self.link.batch_callback(move |_| {
                            play_zipclick();

                            // create an easy task
                            let task = GameTaskBuilder::new("Easy task", TaskKind::Normal, 5, 2);

                            vec![
                                Msg::CloseModal,
                                // create an easy task
                                Msg::EventWithTimeout {
                                    ms: 750,
                                    event: EventBusRequest::AddTask(task),
                                },
                                Msg::EventWithTimeout {
                                    ms: 750 + 600,
                                    event: EventBusRequest::AdvanceTutorial,
                                },
                            ]
                        });

                        html! {
                            <Modal title="Onboarding">
                                { message.body() }
                                <button onclick=click_handler>{ "OK" }</button>
                            </Modal>
                        }
                    }
                    // wait and add a bug task, then advance
                    9 => {
                        let click_handler = self.link.batch_callback(move |_| {
                            play_zipclick();

                            // create an easy task
                            let task = GameTaskBuilder::new("Bug!", TaskKind::Bug, 2, 4);

                            vec![
                                Msg::CloseModal,
                                // create an easy task
                                Msg::EventWithTimeout {
                                    ms: 2_800,
                                    event: EventBusRequest::AddTask(task),
                                },
                                Msg::EventWithTimeout {
                                    ms: 2_800 + 600,
                                    event: EventBusRequest::AdvanceTutorial,
                                },
                            ]
                        });

                        html! {
                            <Modal title="Onboarding">
                                { message.body() }
                                <button onclick=click_handler>{ "OK" }</button>
                            </Modal>
                        }
                    }
                    // wait and add a chore task, then advance
                    11 => {
                        let click_handler = self.link.batch_callback(move |_| {
                            play_zipclick();

                            // create an easy task
                            let task =
                                GameTaskBuilder::new("Refactor stuff", TaskKind::Chore, 0, 3);

                            vec![
                                Msg::CloseModal,
                                // create an easy task
                                Msg::EventWithTimeout {
                                    ms: 20_000,
                                    event: EventBusRequest::AddTask(task),
                                },
                                Msg::EventWithTimeout {
                                    ms: 21_000,
                                    event: EventBusRequest::AdvanceTutorial,
                                },
                            ]
                        });

                        html! {
                            <Modal title="Onboarding">
                                { message.body() }
                                <button onclick=click_handler>{ "OK" }</button>
                            </Modal>
                        }
                    }
                    // all other, do not advance on close,
                    // should instead wait for certain actions to be performed
                    _ => {
                        let click_handler = self.link.callback(move |_| {
                            play_zipclick();

                            Msg::CloseModal
                        });

                        html! {
                            <Modal title="Onboarding">
                                { message.body() }
                                <button onclick=click_handler>{ "OK" }</button>
                            </Modal>
                        }
                    }
                }
            }
            None => html! {},
        };

        let state = &self.state;

        let backlog_tasks = state
            .tasks_backlog
            .iter()
            .map(|t| self.render_task(t))
            .collect::<Vec<_>>();
        let candidate_tasks = state
            .tasks_candidate
            .iter()
            .map(|t| self.render_task(t))
            .collect::<Vec<_>>();
        let progress_tasks = state
            .tasks_progress
            .iter()
            .map(|t| self.render_task(t))
            .collect::<Vec<_>>();
        let review_tasks = state
            .tasks_review
            .iter()
            .map(|t| self.render_task(t))
            .collect::<Vec<_>>();
        let done_tasks = state
            .tasks_done
            .iter()
            .filter(|t| t.visible)
            .map(|t| self.render_task(t))
            .collect::<Vec<_>>();

        let month = self.state.month;
        let time = self.state.time_in_month;

        let hr_desc = match self.state.humans.iter().filter(|h| !h.quit).count() {
            0 => format!("no humans (WAT)"),
            1 => format!("1 lead"),
            2 => format!("1 lead + 1 developer"),
            n => format!("1 lead + {} developers", n),
        };

        let bring_humans_up = self.bring_humans_up;

        let humans = self
            .state
            .humans
            .iter()
            .filter(|human| !human.quit)
            .map(|human| html!(<Human id=human.id name=&human.name status=human.status color=&human.color bring_up=bring_humans_up />))
            .collect::<Vec<_>>();

        let pause_handler = self.link.callback(move |_| {
            play_zipclick();
            Msg::Pause
        });

        let normal_speed_handler = self.link.callback(move |_| {
            play_zipclick();
            Msg::SetGameSpeed(GameSpeed::Normal)
        });

        let fast_speed_handler = self.link.callback(move |_| {
            play_zipclick();
            Msg::SetGameSpeed(GameSpeed::Fast)
        });

        let faster_speed_handler = self.link.callback(move |_| {
            play_zipclick();
            Msg::SetGameSpeed(GameSpeed::Faster)
        });

        let sound_handler = self.link.callback(move |_| Msg::ToggleSound);

        let (class_paused, class_normal, class_fast, class_faster) =
            match self.watch.current_speed() {
                None => ("speed-paused", "", "speed-fast", "speed-faster"),
                Some(GameSpeed::Normal) => ("", "speed-set", "speed-fast", "speed-faster"),
                Some(GameSpeed::Fast) => ("", "", "speed-fast speed-set", "speed-faster"),
                Some(GameSpeed::Faster) => ("", "", "speed-fast", "speed-faster speed-set"),
            };
        
        let sound_icon = if self.sound_enabled {
            "🕪"
        } else {
            "🕨"
        };

        let sound_tooltip = if self.sound_enabled {
            "Audio is enabled; press to disable"
        } else {
            "Audio is disabled; press to enable"
        };

        html! {
            <>
                <div class="status-top">
                    <button class="btn-sound" title=sound_tooltip onclick=sound_handler>{sound_icon}</button>
                    { "Month " } {month}
                    <button class=class_paused onclick=pause_handler>{"⏸"}</button>
                    <button class=class_normal onclick=normal_speed_handler>{"▶"}</button>
                    <button class=class_fast onclick=fast_speed_handler>{"▶▶"}</button>
                    <button class=class_faster onclick=faster_speed_handler>{"▶▶▶"}</button>
                    <div class="status-score">{self.state.total_score / 1_000}</div>
                    <Clock time=time />
                </div>
                <Board product_name=self.state.product_name.clone()>
                    <Stage id=StageId::Backlog description="Backlog">
                        {backlog_tasks}
                    </Stage>
                    <Stage id=StageId::Candidate description="Sprint candidate">
                        {candidate_tasks}
                    </Stage>
                    <Stage id=StageId::Progress description="In progress">
                        {progress_tasks}
                    </Stage>
                    <Stage id=StageId::Review description="Under review">
                        {review_tasks}
                    </Stage>
                    <Stage id=StageId::Done description="Done">
                        {done_tasks}
                    </Stage>
                </Board>
                <div class="human-resources">
                    <div class="human-resources-header">{ "Human Resources: " } { hr_desc }</div>
                    // render humans based on state
                    { humans }
                </div>

                {modal}
            </>
        }
    }

    fn destroy(&mut self) {
        // try to save game
        gloo_console::log!("Saving game...");
        self.state.save().expect_throw("Could not save game");
    }
}

impl Game {
    fn start_watch(&mut self) {
        let link = self.link.clone();
        let tick_fn = move || link.send_message(Msg::Event(EventBusRequest::Tick));
        self.watch.start_with(tick_fn);
    }

    fn update_speed(&mut self, speed: GameSpeed) {
        let link = self.link.clone();
        let tick_fn = move || link.send_message(Msg::Event(EventBusRequest::Tick));
        self.watch.set_speed(speed, tick_fn);
    }

    fn assigned_of(&self, task: &GameTask) -> Option<(u32, String, String)> {
        if let Some(id) = task.assigned {
            let human = self.state.human_of(id);
            human.map(|h| (h.id, h.name.to_string(), h.color.to_string()))
        } else {
            None
        }
    }

    fn render_task(&self, t: &GameTask) -> Html {
        let assigned = self.assigned_of(&t);

        let deadline_ratio = t.deadline
            .map(|deadline| {
                let expected_time = self.state.time.saturating_sub(t.created).max(1);
                let time_left = deadline.saturating_sub(self.state.time);
                time_left as f32 / expected_time as f32
            });

        html!(<Task key=t.id
            id=t.id kind=t.kind stage=t.stage assigned=assigned
            bugs_found=t.bugs_found score=t.score progress=t.progress
            deadline_ratio=deadline_ratio />)
    }
}
