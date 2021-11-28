use gloo_timers::callback::Interval;
use rand::{Rng, SeedableRng};
use rand_distr::{self, Distribution};
use rand_pcg::Pcg32;
use std::fmt::{Debug, Display};
use wasm_bindgen::UnwrapThrowExt;

use crate::{
    components::{
        human::GameHuman,
        task::{GameTask, GameTaskBuilder, TaskKind},
    },
    state::WorldState,
};

pub const BASE_MILLISECONDS_PER_TICK: u32 = 200;

#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum GameSpeed {
    /// 1x game speed
    Normal,
    /// 2x game speed
    Fast,
    /// 4x game speed
    Faster,
}

impl GameSpeed {
    /// The number of milliseconds to wait between ticks.
    pub fn milliseconds_per_tick(self) -> u32 {
        match self {
            GameSpeed::Normal => BASE_MILLISECONDS_PER_TICK,
            GameSpeed::Fast => BASE_MILLISECONDS_PER_TICK / 2,
            GameSpeed::Faster => BASE_MILLISECONDS_PER_TICK / 4,
        }
    }
}

/// The time watch service, emits ticks at a fixed interval when started.
pub struct GameWatch {
    interval: Option<Interval>,
    speed: GameSpeed,
}

impl Debug for GameWatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameWatch")
            .field("interval", &self.interval)
            .finish()
    }
}

impl GameWatch {
    pub fn new() -> Self {
        GameWatch {
            interval: None,
            speed: GameSpeed::Normal,
        }
    }

    pub fn start_with<F>(&mut self, tick_fn: F)
    where
        F: 'static + FnMut() + Clone,
    {
        if self.interval.is_some() {
            self.pause();
            return;
        }

        let ms = self.speed.milliseconds_per_tick();
        let interval = Interval::new(ms, tick_fn);
        self.interval = Some(interval);
    }

    /// Set the new game speed,
    /// unpausing if necessary.
    pub fn set_speed<F>(&mut self, speed: GameSpeed, tick_fn: F)
    where
        F: 'static + FnMut() + Clone,
    {
        if self.current_speed() == Some(speed) {
            // already running at that speed
            return;
        }

        self.pause();

        // replace existing interval
        let interval = Interval::new(speed.milliseconds_per_tick(), tick_fn);

        self.interval = Some(interval);
        self.speed = speed;
    }

    pub fn pause(&mut self) {
        if let Some(interval) = self.interval.take() {
            interval.cancel();
        }
    }

    /// Get the current status of game speed,
    /// returns `None` if the game is paused.
    pub fn current_speed(&self) -> Option<GameSpeed> {
        self.interval.as_ref().map(|_| self.speed)
    }
}

impl Display for GameSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameSpeed::Normal => write!(f, "normal"),
            GameSpeed::Fast => write!(f, "fast"),
            GameSpeed::Faster => write!(f, "faster"),
        }
    }
}

/// Game construct that produces events over game time.
#[derive(Debug)]
pub struct EventReactor {
    /// the random number generator
    rng: Pcg32,
}

/// Some major event that can happen over time.
#[derive(Debug)]
pub enum GameEvent {
    /// extra technical debt
    ExtraTechnicalDebt { message: u32, extra_complexity: u32 },
    /// extraordinary features were requested
    MajorFeatureRequested {
        message: u32,
        tasks: Vec<GameTaskBuilder>,
    },
    /// a bug was reported by clients
    BugReported(GameTaskBuilder),
    /// Just show a random report
    RandomReport(u32),
}

impl EventReactor {
    pub fn new() -> Self {
        EventReactor {
            rng: Pcg32::from_entropy(),
        }
    }

    /// Roll for whether the human introduced a bug
    /// while working on the given task.
    pub fn human_introduced_bug(
        &mut self,
        human: &GameHuman,
        task: &GameTask,
        project_complexity: u32,
    ) -> bool {
        let det = 3_500 + human.experience * 16;
        let mut num = task.difficulty * project_complexity / 2;

        // if bugs were found before,
        // that means we're fixing previous bugs,
        // so less chance of introducing new ones
        if task.bugs_found > 0 {
            num /= 5
        };

        self.rng.gen_ratio(num.min(det) as u32, det)
    }

    /// Roll for whether the human discovered a bug
    /// while reviewing a task.
    pub fn human_detected_bug(
        &mut self,
        human: &GameHuman,
        task: &GameTask,
        project_complexity: u32,
    ) -> bool {
        if task.bugs == 0 {
            return false;
        }

        for _ in task.bugs_found..task.bugs {
            let det = 2_000 + task.difficulty * 70 + project_complexity * 60;
            let num = 10 + human.experience / 2;
            // triple the chances if reviewed by someone else
            let num = if task.developed_by != Some(human.id) {
                num * 3
            } else {
                num
            };

            if self.rng.gen_ratio(num.min(det) as u32, det) {
                return true;
            }
        }
        false
    }

    /// Determine how much of the score to deduct.
    pub fn score_damage(&mut self, total_score: u32, score_linger_rate: u32) -> u32 {
        let dist = rand_distr::Normal::new(score_linger_rate as f32, (total_score / 2_000) as f32)
            .unwrap_throw();
        let damage = dist.sample(&mut self.rng);
        // !!! remove in prod
        gloo_console::debug!("lingering score damage:", damage);

        (damage as u32).clamp(0, total_score / 5)
    }

    /// Roll for whether to have a major event
    pub fn major_event(&mut self, state: &WorldState) -> Option<GameEvent> {
        match self.rng.gen_range(1..=1_000) {
            0..=35 => {
                if state.bugs == 0 {
                    return None;
                }

                let task = GameTaskBuilder::new(
                    "",
                    TaskKind::Bug,
                    self.rng.gen_range(0..2),
                    self.rng.gen_range(2..12),
                );

                Some(GameEvent::BugReported(task))
            }
            950..=1_000 => {
                let n_new_tasks = self.rng.gen_range(1..=3);

                let tasks = (0..n_new_tasks)
                    .map(|_| {
                        GameTaskBuilder::new_with_deadline(
                            "Extraordinary task",
                            TaskKind::Normal,
                            // generally higher score than usual
                            self.rng.gen_range(4..=16),
                            // generally harder too
                            self.rng.gen_range(3..15),
                        )
                    })
                    .collect();

                Some(GameEvent::MajorFeatureRequested { message: 0, tasks })
            }
            750..=899 => Some(GameEvent::RandomReport(self.rng.gen_range(0..=8))),
            _ => {
                // do nothing
                None
            }
        }
    }

    pub fn ingest_task(
        &mut self,
        you_experience: u32,
        bugs: u32,
        complexity: u32,
        task_ingest_rate: u32,
        tasks_in_backlog: usize,
    ) -> Option<GameTaskBuilder> {
        let det = 4_400 + task_ingest_rate;
        let num = task_ingest_rate + you_experience;

        // inflate ingestion chance
        let num = match tasks_in_backlog {
            0..=1 => num * 2,
            8..=12 => num / 2,
            13..=20 => num / 4,
            21..=29 => num / 8,
            _ => return None,
        };

        if self.rng.gen_ratio(num.min(det) as u32, det) {
            // weighted sampling
            // - bug finding is weighed on nr of bugs
            // - chore tasks are weighed on complexity
            let n_fraction = 24;
            let b_fraction = bugs;
            let c_fraction = complexity / 2;
            gloo_console::debug!(
                "ingestion fractions - n: ",
                n_fraction,
                " b:",
                b_fraction,
                " c:",
                c_fraction
            );
            let d = n_fraction + b_fraction + c_fraction;

            let i: u32 = self.rng.gen_range(0..d);

            let mut kind = if i < b_fraction {
                TaskKind::Bug
            } else if i >= d - c_fraction {
                TaskKind::Chore
            } else {
                TaskKind::Normal
            };

            if kind == TaskKind::Bug && bugs == 0 {
                kind = TaskKind::Chore;
            }

            let score = match kind {
                TaskKind::Chore => 0,
                TaskKind::Normal => self.rng.gen_range(1..=8),
                TaskKind::Bug => self.rng.gen_range(0..=2),
            };
            let difficulty = match kind {
                TaskKind::Normal => self.rng.gen_range(1..12),
                TaskKind::Bug => self.rng.gen_range(2..12),
                TaskKind::Chore => self.rng.gen_range(5..16),
            };
            let task = GameTaskBuilder::new("", kind, score, difficulty);

            Some(task)
        } else {
            None
        }
    }

    /// A procedure to generate multiple feature tasks
    /// at the beginning of the month.
    pub fn ingest_important_tasks(
        &mut self,
        month: u32,
        task_ingest_rate: u32,
    ) -> Vec<GameTaskBuilder> {
        let dist =
            rand_distr::Normal::new((month + task_ingest_rate) as f32 / 5., 2.).unwrap_throw();

        let num_tasks = (dist.sample(&mut self.rng).round() as u32).max(3);

        let base_difficulty = 4 + month / 4;
        let base_score = 4 + month as i32 / 5;
        (0..num_tasks)
            .map(|_| {
                let score = self.rng.gen_range(base_score..=(base_score + 6));
                let difficulty = self.rng.gen_range(base_difficulty..(base_difficulty + 10));
                GameTaskBuilder::new_with_deadline("", TaskKind::Normal, score, difficulty)
            })
            .collect()
    }

    /// Generate a new human
    pub fn new_human(&mut self, id: u32, month: u32) -> GameHuman {
        let dist = rand_distr::Normal::new(50_f32, 10.).unwrap_throw();
        let experience = dist.sample(&mut self.rng).clamp(35., 80.) as u32;

        let n = month.saturating_sub(3) as usize % HUMAN_NAMES.len();

        GameHuman::new(id, HUMAN_NAMES[n], HUMAN_COLORS[n], experience)
    }
}

static HUMAN_NAMES: [&'static str; 16] = [
    "May", "Ben", "Joan", "Sam", "Kris", "Joe", "Sue", "Tim", "Dory", "Tom", "Anne", "Abe", "Yao",
    "Mary", "Ray", "Jon",
];

static HUMAN_COLORS: [&'static str; 16] = [
    "#00d", "#dd0", "#c6c", "#0cc", "#0dd", "#ccc", "#d00", "#6c0", "#d0d", "#c0c", "#ddc", "#cdd",
    "#c6c", "#3f7", "#c0c", "#f30",
];
