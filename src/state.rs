//! Module for handling the state of the game.
//!
//!
use std::{collections::HashSet, rc::Rc};

use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsValue, UnwrapThrowExt};
use yew::web_sys;

use crate::{
    components::{
        human::{GameHuman, HumanStatus},
        messages::Message,
        stage::StageId,
        task::{GameTask, GameTaskBuilder, TaskKind},
    },
    data_transfer::payload::TaskTransfer,
    event_bus::EventBusRequest,
    services::{EventReactor, GameEvent},
};

/// The number of ticks events in a full game month.
pub const TICKS_PER_MONTH: u32 = 1_000;

/// The number of ticks for for a major tick event
/// to be triggered.
pub const TICKS_PER_MAJOR_TICK: u32 = 250;

/// A magic constant that decides the rate of score lingering based on technical debt
pub const LINGER_FACTOR: u32 = 120;

/// The number of ticks for the score to linger.
pub const TICKS_PER_SCORE_DAMAGE: u32 = 25;

/// A type for a partiular moment in game time.
/// This is the number of ticks since the creation of a new game.
pub type Timestamp = u32;

/// The full state of the game.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct WorldState {
    /// the project or product's name
    pub product_name: Rc<str>,
    /// the current month (round)
    pub month: u32,
    /// the number of ticks which have passed since the beginning of the game
    pub time: Timestamp,
    /// the number of ticks which have passed since the beginning of the current month
    pub time_in_month: u32,

    /// the ID of the next task to create
    pub next_task_id: u32,

    /// the player's current score in milliparts of a unit
    /// (all scores presented in tasks are in units,
    /// but score damage works in 1/1000ths)
    pub total_score: u32,

    /// the player's score gained or lost this month,
    /// in milliparts of a unit
    pub score_in_month: i32,

    /// a hidden count of bugs in the software,
    /// used to recalculate the score linger rate
    pub bugs: u32,

    /// the number of bugs found and fixed over the course of the entire game
    pub bugs_fixed_in_total: u32,

    /// the number of bugs found and fixed over the course of this month
    pub bugs_fixed_in_month: u32,

    /// the product software's technical debt,
    /// which affects development efforts and the score linger rate
    pub complexity: u32,

    /// the player's score lingering rate,
    /// which affects how much score is damaged over time
    pub score_linger_rate: u32,

    /// the rate at which You can devise new tasks
    pub task_ingest_rate: u32,

    /// the list of tasks in backlog
    pub tasks_backlog: Vec<GameTask>,

    /// the list of tasks in sprint candidate
    pub tasks_candidate: Vec<GameTask>,

    /// the list of tasks in progress
    pub tasks_progress: Vec<GameTask>,

    /// the list of tasks under review
    pub tasks_review: Vec<GameTask>,

    /// the list of tasks done
    pub tasks_done: Vec<GameTask>,

    /// A list of all human resources
    pub humans: Vec<GameHuman>,

    /// The current tutorial phase, if currently in the tutorial.
    /// Starts at 0, the first message is given at 1.
    pub tutorial: Option<u32>,
}

/// The outcome of a game event request.
#[derive(Debug)]
pub enum EventOutcome {
    /// No game state changes occurred, no need to re-render
    Nothing,

    /// The game state has changed,
    /// should issue a re-render and continue normally.
    Update,

    /// A message to the user should appear
    OpenMessage(Message),

    EndOfMonth(MonthlyReport),

    /// Alert the user with this message,
    /// likely because the requested operation is invalid.
    Alert(&'static str),
}

#[derive(Debug)]
pub struct MonthlyReport {
    /// month number
    pub month: u32,
    /// score this month, in units
    pub score: i32,
    /// total score, in units
    pub total_score: u32,
    /// number of tasks done this month
    pub tasks_done: usize,
    /// number of bugs fixed this month
    pub bugs_fixed: u32,
    /// current project complexity
    pub complexity: u32,
}

impl WorldState {
    /// Create the world state of a brand new game
    pub fn new(product_name: String, tutorial: bool) -> Self {
        Self {
            product_name: product_name.into(),
            month: 0,
            time: 0,
            time_in_month: 0,
            next_task_id: 351,
            total_score: 0,
            score_in_month: 0,
            bugs: 0,
            bugs_fixed_in_month: 0,
            bugs_fixed_in_total: 0,
            complexity: 15,
            score_linger_rate: 0,
            tasks_backlog: vec![],
            tasks_candidate: vec![],
            tasks_progress: vec![],
            tasks_review: vec![],
            tasks_done: vec![],
            humans: vec![GameHuman::new(0, "You", "#fff", 50)],
            // do not ingest tasks during tutorial
            task_ingest_rate: if tutorial { 0 } else { 10 },
            tutorial: if tutorial { Some(0) } else { None },
        }
    }

    /// Load the whole state of the game from local storage.
    pub fn load_from_storage() -> Result<Option<Self>, JsValue> {
        let local_storage = try_local_storage()?;

        let save_item = local_storage.get_item("save")?;

        if let Some(save_item) = save_item {
            let world_state =
                serde_json::from_str(&save_item).map_err(|e| JsValue::from(e.to_string()))?;

            gloo_console::log!("Game successfully loaded from local storage");
            Ok(Some(world_state))
        } else {
            Ok(None)
        }
    }

    pub fn has_save_in_storage() -> Result<bool, JsValue> {
        let local_storage = try_local_storage()?;

        Ok(local_storage.get_item("save")?.is_some())
    }

    pub fn save(&self) -> Result<(), JsValue> {
        let local_storage = try_local_storage()?;

        let state_payload =
            serde_json::to_string(&self).map_err(|e| JsValue::from(e.to_string()))?;
        local_storage.set_item("save", &state_payload)?;
        gloo_console::log!("Game saved to local storage");
        Ok(())
    }

    pub fn dummy() -> Self {
        let mut state = dummy_state();
        state.update_score_linger_rate();
        state
    }

    /// Merge the given task upstream,
    /// applying changes to state as necessary.
    pub fn merge_task(&mut self, task_transfer: &TaskTransfer) {
        let mut task = self
            .find_task_by_transfer_mut(&task_transfer)
            .expect_throw("could not find task to merge");

        // unassign it from the human
        task.assigned = None;
        // remove progress
        task.progress = 0.;

        // get task properties for calculations
        let kind = task.kind;
        let bugs = task.bugs;
        let difficulty = task.difficulty;
        let task_score = task.score * 1_000;

        // add score
        self.total_score = ((self.total_score as i32).saturating_add(task_score).max(0)) as u32;
        self.score_in_month += task_score;

        // add bugs
        self.bugs += bugs;

        // add complexity
        match kind {
            TaskKind::Bug => {
                self.complexity += difficulty / 5;
                // remove bug that was upstream
                self.bugs -= 1;
            }
            TaskKind::Normal => {
                self.complexity += 1 + difficulty / 4;
            }
            TaskKind::Chore => {
                self.complexity = self.complexity.saturating_sub(2 + difficulty / 4);
            }
        }

        // update score linger rate
        self.update_score_linger_rate();
    }

    fn update_score_linger_rate(&mut self) {
        // update based on bugs and complexity
        // TODO refine
        self.score_linger_rate =
            (self.bugs + (self.complexity * LINGER_FACTOR) / 2).saturating_sub(10);

        gloo_console::debug!(
            "Complexity: ",
            self.complexity,
            "; Linger rate: ",
            self.score_linger_rate
        );
    }

    pub fn month(&self) -> u32 {
        self.month
    }

    pub fn human_of(&self, id: u32) -> Option<&GameHuman> {
        self.humans
            .binary_search_by_key(&id, |h| h.id)
            .ok()
            .map(|i| &self.humans[i])
    }

    /// Apply the given request to the world state.
    ///
    /// Returns false if the request was invalid.
    pub fn apply_event(
        &mut self,
        event: EventBusRequest,
        reactor: &mut EventReactor,
    ) -> EventOutcome {
        // handle all state change requests here
        // (better move specific state operations to state module though)
        match event {
            EventBusRequest::MoveTask { task, to } => self.handle_move_task(task, to),
            EventBusRequest::AssignTask { task, human_id } => self.assign_task(task, human_id),
            EventBusRequest::Tick => self.tick(reactor),
            EventBusRequest::AdvanceTutorial => self.advance_tutorial(),
            EventBusRequest::AddTask(task) => {
                self.add_task(task);
                EventOutcome::Update
            }
            EventBusRequest::NextMonth => self.next_month(),
            _ => EventOutcome::Nothing,
        }
    }

    fn find_task_by_transfer(&self, transfer: &TaskTransfer) -> Option<&GameTask> {
        let tasks = match transfer.from {
            StageId::Backlog => &self.tasks_backlog,
            StageId::Candidate => &self.tasks_candidate,
            StageId::Progress => &self.tasks_progress,
            StageId::Review => &self.tasks_review,
            StageId::Done => &self.tasks_done,
        };

        tasks.iter().find(|t| t.id == transfer.id)
    }

    fn find_task_by_transfer_mut(&mut self, transfer: &TaskTransfer) -> Option<&mut GameTask> {
        let tasks = match transfer.from {
            StageId::Backlog => &mut self.tasks_backlog,
            StageId::Candidate => &mut self.tasks_candidate,
            StageId::Progress => &mut self.tasks_progress,
            StageId::Review => &mut self.tasks_review,
            StageId::Done => &mut self.tasks_done,
        };

        tasks.iter_mut().find(|t| t.id == transfer.id)
    }

    fn move_task(&mut self, task: &TaskTransfer, to: StageId) {
        let task_list = match task.from {
            StageId::Backlog => &mut self.tasks_backlog,
            StageId::Candidate => &mut self.tasks_candidate,
            StageId::Progress => &mut self.tasks_progress,
            StageId::Review => &mut self.tasks_review,
            StageId::Done => &mut self.tasks_done,
        };

        let index = task_list
            .iter()
            .position(|t| t.id == task.id)
            .unwrap_throw();

        // remove it from old list
        let mut task = task_list.remove(index);
        task.stage = to;

        // place it on new list
        let new_task_list = match to {
            StageId::Backlog => &mut self.tasks_backlog,
            StageId::Candidate => &mut self.tasks_candidate,
            StageId::Progress => &mut self.tasks_progress,
            StageId::Review => &mut self.tasks_review,
            StageId::Done => &mut self.tasks_done,
        };

        new_task_list.push(task);
    }

    fn handle_move_task(&mut self, mut task: TaskTransfer, to: StageId) -> EventOutcome {
        let game_task = self.find_task_by_transfer_mut(&task).unwrap_throw();

        match (game_task.stage, to) {
            // unconditional:
            // from backlog to candidate
            (StageId::Backlog, StageId::Candidate) => {
                self.move_task(&task, to);
                EventOutcome::Update
            }
            // from in progress to candidate,
            // dev progress is retained
            (StageId::Progress, StageId::Candidate) => {
                self.move_task(&task, to);
                EventOutcome::Update
            }
            // from under review to in progress
            (StageId::Review, StageId::Progress) => {
                if game_task.bugs_found > 0 {
                    game_task.progress = 0.66666;
                }
                self.move_task(&task, to);

                EventOutcome::Update
            }
            // from under review to done
            (StageId::Review, StageId::Done) => {
                gloo_console::debug!("review -> done");
                self.move_task(&task, to);
                task.from = StageId::Done;
                self.merge_task(&task);

                // tutorial step
                if let Some(6 | 7 | 8) = self.tutorial {
                    self.tutorial = Some(8);
                    return self.advance_tutorial();
                }

                EventOutcome::Update
            }
            // only if the task is fully specified
            // and assigned to a developer:
            // from candidate to in progress
            (StageId::Candidate, StageId::Progress)
                if game_task.is_specified() && game_task.assigned.is_some() =>
            {
                // progress now means development progress
                game_task.progress = 0.;
                self.move_task(&task, to);

                EventOutcome::Update
            }
            // only if fully developed:
            // from in progress to under review
            (StageId::Progress, StageId::Review) => {
                if game_task.is_developed() {
                    self.move_task(&task, to);

                    // tutorial step 7
                    if self.tutorial == Some(7) {
                        return self.advance_tutorial();
                    }

                    EventOutcome::Update
                } else {
                    EventOutcome::Nothing
                }
            }
            // only if fully developed:
            // from in progress to done
            (StageId::Progress, StageId::Done) => {
                if game_task.is_developed() {
                    self.move_task(&task, to);
                    task.from = StageId::Done;
                    self.merge_task(&task);

                    // tutorial step
                    if let Some(6 | 7 | 8) = self.tutorial {
                        self.tutorial = Some(8);
                        return self.advance_tutorial();
                    }

                    EventOutcome::Update
                } else {
                    EventOutcome::Nothing
                }
            }
            // only if not yet specified:
            // from candidate to backlog
            (StageId::Candidate, StageId::Backlog) if !game_task.is_specified() => {
                self.move_task(&task, to);
                EventOutcome::Update
            }
            (_, _) => {
                // not a valid move
                EventOutcome::Nothing
            }
        }
    }

    fn assign_task(&mut self, task: TaskTransfer, human_id: u32) -> EventOutcome {
        let task = self.find_task_by_transfer_mut(&task).unwrap_throw();

        if let Some(assigned_human) = task.assigned {
            if assigned_human == human_id {
                // already assigned to this human, do nothing
                return EventOutcome::Nothing;
            }
        }

        // assign at this task
        task.assigned = Some(human_id);

        EventOutcome::Update
    }

    fn tick(&mut self, reactor: &mut EventReactor) -> EventOutcome {
        // move time forward
        self.time += 1;
        self.time_in_month += 1;

        let mut worked = HashSet::new();

        let mut score_penalty = 0;
        let time = self.time;
        // detect unfulfilled tasks
        for task in self.all_tasks_iter_mut() {
            if let Some(deadline) = task.deadline {
                if deadline < time {
                    
                    // apply penalty
                    score_penalty += task.score * 1_000;

                    // reset deadline
                    task.deadline = None;
                }
            }
        }
        if score_penalty > 0 {
            self.total_score = self.total_score.saturating_sub(score_penalty as u32);
            self.score_in_month -= score_penalty as i32;
        }

        // apply human work (development)
        for task in &mut self.tasks_progress {
            if task.is_developed() {
                continue;
            }

            if let Some(human_id) = task.assigned {
                if worked.contains(&human_id) {
                    // this human already worked
                    continue;
                }

                let human = self.humans.get_mut(human_id as usize).unwrap_throw();

                // do progress on task
                let added_progress = 0.005
                    + (5 + human.experience) as f64
                        / (task.difficulty * 60 + self.complexity * 55) as f64;
                let complete = task.add_progress(added_progress);
                human.status = HumanStatus::Coding;

                // as this human worked on the task,
                // they cannot work on other things
                worked.insert(human_id);

                // roll for adding a bug
                if reactor.human_introduced_bug(human, task, self.complexity) {
                    task.bugs += 1;

                    // !!! remove in prod
                    gloo_console::debug!("Bug introduced in", format!("T{}", task.id));
                }

                if complete {
                    // record developed_by
                    task.developed_by = Some(human.id);
                    // record bugs fixed
                    self.bugs_fixed_in_month += task.bugs_found;
                    // clear bugs found
                    task.bugs -= task.bugs_found;
                    task.bugs_found = 0;

                    // add experience to human
                    human.experience = (human.experience + task.difficulty / 4).min(128);

                    if self.tutorial == Some(5) {
                        // ensure bug in task,
                        task.bugs = task.bugs.max(1);
                        // increase experience of You
                        // (to make bug a bit easier to find)
                        self.humans[0].experience += 1;
                        // advance tutorial
                        return self.advance_tutorial();
                    }
                }
            }
        }

        // traverse the tasks again for specification
        for task in &mut self.tasks_candidate {
            // if You already worked,
            // then you cannot work on task specification
            if worked.contains(&0) {
                break;
            }
            // work on specification
            if task.is_specified() {
                continue;
            }
            let you = self.humans.get_mut(0).unwrap_throw();

            // do writing progress on task
            let added_progress = (1 + you.experience) as f64 / (task.difficulty * 128) as f64;
            let complete = task.add_progress(added_progress);
            you.status = HumanStatus::Writing;

            worked.insert(0);

            if complete && (self.tutorial == Some(4) || self.tutorial == Some(10)) {
                // advance tutorial
                return self.advance_tutorial();
            }
        }

        // traverse tasks under review
        for task in &mut self.tasks_review {
            if task.stage != StageId::Review {
                continue;
            }

            if let Some(human_id) = task.assigned {
                if worked.contains(&human_id) {
                    // this human already worked
                    continue;
                }

                let human = self.humans.get_mut(human_id as usize).unwrap_throw();

                // review (detect bugs)
                human.status = HumanStatus::Reviewing;
                if reactor.human_detected_bug(human, task, self.complexity) {
                    task.bugs_found += 1;
                    // !!! remove in prod
                    gloo_console::debug!("Bug found in", format!("T{}", task.id));

                    // tutorial step when reviewing the first task
                    if self.tutorial == Some(6) {
                        return self.advance_tutorial();
                    }
                }

                // as this human worked on the task,
                // they cannot work on other things
                worked.insert(human_id);
            }
        }

        if !worked.contains(&0) && self.tutorial.filter(|&phase| phase < 12).is_none() {
            // You are idle,
            // so you will think about tasks to do

            // roll for ingestion of new task
            // based on ingestion rate and experience of You
            let you_experience = self.humans[0].experience;

            if let Some(new_task) = reactor.ingest_task(
                you_experience,
                self.bugs,
                self.complexity,
                self.task_ingest_rate,
                self.tasks_backlog.len(),
            ) {
                let id = self.add_task(new_task);
                gloo_console::debug!("Task", id, "ingested");
            }
        }

        // update status of humans who did not work
        for human in &mut self.humans {
            if !worked.contains(&human.id) {
                human.status = HumanStatus::Idle;
            }
        }

        // check for start of month
        if self.time_in_month == 5 {
            if let Some(outcome) = self.start_of_month(reactor) {
                return outcome;
            }
        }

        // check for end of month
        if self.time_in_month >= TICKS_PER_MONTH {
            // trigger end of month
            let report = self.month_report();
            return EventOutcome::EndOfMonth(report);
        }

        // advance tutorial to end step if close to end of month
        if self.time_in_month >= TICKS_PER_MONTH - TICKS_PER_MONTH / 16
            && self.tutorial.filter(|&phase| phase < 13).is_some()
        {
            self.tutorial = Some(12);
            return self.advance_tutorial();
        }

        if (self.time % TICKS_PER_MAJOR_TICK) == 0 && self.tutorial.is_none() {
            // roll for random events
            match reactor.major_event(&self) {
                Some(GameEvent::RandomReport(report_id)) => {
                    // open modal with some useless message
                    let message = Message::random_report(report_id);
                    return EventOutcome::OpenMessage(message);
                }
                Some(GameEvent::BugReported(task)) => {
                    // add bug task
                    self.add_task(task);
                    // deduct score
                    self.score_in_month -= 2_000;
                    self.total_score = self.total_score.saturating_sub(2_000);

                    // open modal with a bug message
                    return EventOutcome::OpenMessage(Message::bug_reported());
                }

                Some(GameEvent::ExtraTechnicalDebt {
                    message,
                    extra_complexity,
                }) => {
                    self.complexity += extra_complexity;
                    self.update_score_linger_rate();
                    let message = Message::extra_technical_debt();
                    return EventOutcome::OpenMessage(message);
                }

                Some(GameEvent::MajorFeatureRequested { message, tasks }) => {
                    self.add_tasks(tasks);

                    // open modal with a major feature request message
                    return EventOutcome::OpenMessage(Message::feature_requested());
                }

                None => {
                    // do nothing
                }
            }

            if (self.time_in_month % TICKS_PER_SCORE_DAMAGE) == 0 {
                // roll for score damage
                let damage = reactor.score_damage(self.total_score, self.score_linger_rate);

                gloo_console::debug!("Score damage to apply: ", damage);

                self.score_in_month -= damage as i32;
                self.total_score = self.total_score.saturating_sub(damage);
            }
        }

        EventOutcome::Update
    }

    /// Advance to the next month
    fn next_month(&mut self) -> EventOutcome {
        // safe-guard: ignore request if month is not over
        if self.time_in_month < TICKS_PER_MONTH {
            return EventOutcome::Nothing;
        }

        // reset time in month
        self.month += 1;
        self.time_in_month = 0;

        // reset score this month
        self.score_in_month = 0;

        // reset bugs fixed in month
        self.bugs_fixed_in_month = 0;

        // update task ingestion rate
        self.task_ingest_rate += 1;

        // hide tasks done
        for t in &mut self.tasks_done {
            t.visible = false;
        }

        // reset tutorial
        if self.tutorial.is_some() {
            self.tutorial = None;

            // remove onboard guy
            if let Some(guy) = self.humans.iter_mut().find(|h| h.name == "Guy") {
                guy.quit = true;
                let guy_id = guy.id;

                // unassign tasks everywhere
                for t in self.all_tasks_iter_mut() {
                    if t.assigned == Some(guy_id) {
                        t.assigned = None;
                    }
                }
            }
        }

        EventOutcome::Update
    }

    fn start_of_month(&mut self, reactor: &mut EventReactor) -> Option<EventOutcome> {
        if self.tutorial.is_some() {
            // do nothing if tutorial is active
            return None;
        }

        // ingest a bunch of important tasks at once
        self.add_tasks(reactor.ingest_important_tasks(self.month, self.task_ingest_rate));

        // check whether it is time to introduce another human
        let humans_count = self.humans.iter().filter(|h| !h.quit).count();

        let expected_humans = 1 + (self.month + 3) / 6;

        if expected_humans as usize > humans_count {
            // introduce a new human
            let new_human = reactor.new_human(self.next_human_id(), self.month);

            let message = Message::new_human(&new_human);

            self.humans.push(new_human);

            return Some(EventOutcome::OpenMessage(message));
        }

        Some(EventOutcome::Update)
    }

    fn advance_tutorial(&mut self) -> EventOutcome {
        if let Some(phase) = &mut self.tutorial {
            *phase += 1;

            if *phase == 11 {
                // add onboard guy to team
                let guy = GameHuman::new(1, "Guy", "#333", 126);
                self.humans.push(guy);
            }

            EventOutcome::OpenMessage(Message::Tutorial(*phase))
        } else {
            EventOutcome::Nothing
        }
    }

    fn next_task_id(&self) -> u32 {
        self.next_task_id
    }

    pub fn next_human_id(&self) -> u32 {
        self.humans.last().map(|t| t.id + 1).unwrap_or(0)
    }

    /// get an iterator for all non-merged tasks, mutable
    fn all_tasks_iter_mut(&mut self) -> impl Iterator<Item = &mut GameTask> {
        self.tasks_backlog
            .iter_mut()
            .chain(self.tasks_candidate.iter_mut())
            .chain(self.tasks_progress.iter_mut())
            .chain(self.tasks_review.iter_mut())
    }

    fn month_report(&self) -> MonthlyReport {
        MonthlyReport {
            month: self.month,
            total_score: self.total_score / 1000,
            score: self.score_in_month / 1000,
            tasks_done: self.tasks_done.iter().filter(|t| t.visible).count(),
            bugs_fixed: self.bugs_fixed_in_month,
            complexity: self.complexity,
        }
    }

    fn add_task(&mut self, task: GameTaskBuilder) -> u32 {
        let id = self.next_task_id;
        self.next_task_id += 1;
        let created = self.time;
        let GameTaskBuilder {
            description,
            kind,
            score,
            difficulty,
            max_time,
        } = task;
        let task = if let Some(max_time) = max_time {
            let deadline = self.time + max_time;
            GameTask::new_with_deadline(id, created, deadline, description, kind, score, difficulty)
        } else {
            GameTask::new(id, created, description, kind, score, difficulty)
        };
        self.tasks_backlog.push(task);
        id
    }

    fn add_tasks(&mut self, task: impl IntoIterator<Item = GameTaskBuilder>) {
        for t in task {
            self.add_task(t);
        }
    }
}

#[derive(Debug)]
pub enum TutorialEvent {}

/// Save the whole state of the game onto local storage.
pub fn save_to_storage(world_state: &WorldState) -> Result<(), JsValue> {
    let local_storage = try_local_storage()?;

    let data = serde_json::to_string(world_state).map_err(|e| JsValue::from(e.to_string()))?;

    local_storage.set_item("save", &data)?;

    Ok(())
}

/// Gracefully try to obtain the Web local storage API.
pub fn try_local_storage() -> Result<web_sys::Storage, JsValue> {
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("Could not obtain window"))
        .and_then(|window| {
            window
                .local_storage()
                .and_then(|x| x.ok_or_else(|| JsValue::from_str("Could not obtain local storage")))
        })
}

#[allow(unused)]
fn dummy_state() -> WorldState {
    WorldState {
        product_name: "Test Product".to_owned().into(),
        month: 6,
        time: 6 * TICKS_PER_MONTH + 200,
        time_in_month: 200,
        next_task_id: 5,
        bugs: 4,
        bugs_fixed_in_month: 0,
        bugs_fixed_in_total: 10,
        total_score: 420_500,
        score_in_month: 20_500,
        complexity: 30,
        task_ingest_rate: 20,
        score_linger_rate: 0, // not relevant, will be recalculated
        tasks_backlog: vec![
            GameTask::new(1, 6_000, "Test tasks in general", TaskKind::Normal, 6, 20),
            GameTask::new(2, 6_000, "Test bugs in general", TaskKind::Bug, 2, 12),
            GameTask::new(3, 6_000, "Test chores in general", TaskKind::Chore, 0, 10),
        ],
        tasks_candidate: vec![],
        tasks_progress: vec![GameTask {
            id: 4,
            created: 5_200,
            deadline: None,
            description: "Test a task in progress".to_string(),
            kind: TaskKind::Normal,
            stage: StageId::Progress,
            assigned: None,
            developed_by: None,
            score: 7,
            difficulty: 10,
            progress: 0.25,
            specified: true,
            bugs: 1,
            bugs_found: 0,
            visible: true,
        }],
        tasks_review: vec![],
        tasks_done: vec![],
        humans: vec![
            GameHuman::new(0, "You", "#fff", 100),
            GameHuman {
                id: 1,
                name: "Guy".into(),
                color: "#333".into(),
                status: crate::components::human::HumanStatus::Idle,
                experience: 100,
                progress: 0.,
                quit: false,
            },
        ],
        tutorial: None,
    }
}
