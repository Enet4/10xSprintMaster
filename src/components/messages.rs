use std::collections::HashMap;

use yew::{html, Html};

use crate::state::MonthlyReport;

use super::human::GameHuman;

#[derive(Debug)]
pub enum Message {
    /// A simple message with only a title and message.
    Simple { title: String, message: String },

    /// A tutorial message.
    Tutorial(u32),

    /// End of month message.
    EndOfMonth(MonthlyReport),
}

impl Message {
    /// Create a new simple message.
    pub fn new_simple(title: impl Into<String>, message: impl Into<String>) -> Self {
        Message::Simple {
            title: title.into(),
            message: message.into(),
        }
    }

    /// Create a new simple message with template variable replacements (e.g. `$PROJECT_NAME`).
    pub fn new_simple_with_vars(
        title: impl Into<String>,
        message: impl Into<String>,
        vars: &HashMap<&'static str, String>,
    ) -> Self {
        let mut message: String = message.into();

        for (key, var_value) in vars {
            message = message.replace(key, var_value);
        }

        Message::Simple {
            title: title.into(),
            message,
        }
    }

    pub fn random_report(report_id: u32) -> Self {
        let title = "General message via chat".to_string();

        let message = match report_id {
            0 => "Today is pizza day! 🍕 Don't forget to mark your preference in the #lunch channel!",
            1 => "Don't forget that next Wednesday is Meme day. Post your memes on the #memes channel.",
            2 => "Hey folks, let's go grab some coffee! ☕",
            3 => "I heard it's been a rough night for the DevOps team. I wonder why we still have a single DevOps team in the first place...",
            4 => "Things have been complicated over at DevOps. Lend a hand if you can.",
            _ => "Sorry, my mistake. There is no message for you.",
        };
        Message::new_simple(title, message)
    }

    pub fn bug_reported() -> Self {
        let title = "A message from the board of directors".to_string();

        let body = "Clients are complaining about a problem with the software. This is crippling our image. Please fix it as soon as possible.";
        Message::new_simple(title, body)
    }

    pub fn extra_technical_debt() -> Self {
        let title = "Emergency dev meeting report".to_string();
        let body = "Team members have called out that one of the key dependencies is very outdated, and are having trouble working with this version. Consider placing more efforts in migrating dependencies.";

        Message::new_simple(title, body)
    }

    pub fn feature_requested() -> Self {
        let title = "A message from the board of directors".to_string();
        let body = "Our favorite client has requested a feature. Please be sure to work on it in due time.";

        Message::new_simple(title, body)
    }

    pub fn new_human(human: &GameHuman) -> Self {
        let title = "A message from the board of directors".to_string();
        let body = format!(
            "{} has been hired, and is now part of your development team!",
            human.name
        );

        Message::new_simple(title, body)
    }

    pub fn body(&self) -> Html {
        match self {
            Message::Simple { title: _, message } => {
                html! {
                    <p>{message}</p>
                }
            }
            Message::EndOfMonth(report) => end_of_month(report),
            Message::Tutorial(phase) => tutorial(*phase),
        }
    }
}

fn extra_technical_debt(message: u32) -> Html {
    match message {
        _ => html! {
            <p>
            </p>
        },
    }
}

fn end_of_month(report: &MonthlyReport) -> Html {
    let complexity = match report.complexity {
        0..=7 => "very low",
        8..=15 => "low",
        16..=40 => "manageable",
        41..=64 => "high",
        65..=70 => "very high",
        _ => "unbearable",
    };

    html! {
        <ul class="month-report">
            <li><strong>{"Score gained: "}</strong><span>{report.score}</span></li>
            <li><strong>{"Total score: "}</strong><span>{report.total_score}</span></li>
            <li><strong>{"Tasks done: "}</strong><span>{report.tasks_done}</span></li>
            <li><strong>{"Bugs fixed: "}</strong><span>{report.bugs_fixed}</span></li>
            <li><strong>{"Technical debt: "}</strong><span>{complexity}</span></li>
        </ul>
    }
}

fn tutorial(phase: u32) -> Html {
    let text_node = match phase {
        1 => html! {
            <p>
                {"Hey there! So I heard you are going to replace me as the next development lead next month. 🙂 "}
                {"I'll give you an overview of the code base"}
                {" and explain how to coordinate a team once you have more developers involved."}
            </p>
        },
        2 => html! {
            <>
                <p>
                    {"Behind me is the workboard that the company is using "}
                    {"to keep track of tasks and understand how much progress has been done in them."}
                </p>
                <p>
                    {"There are five stages every task must go through, in this order:"}
                    <ol>
                        <li><b>{"Backlog: "}</b> {"all tasks are devised here"}</li>
                        <li><b>{"Sprint candidate: "}</b> {"for tasks to be worked on soon"}</li>
                        <li><b>{"In progress: "}</b> {"for tasks with development work in progress"}</li>
                        <li><b>{"Under review: "}</b> {"to double check that the changes are in good condition"}</li>
                        <li><b>{"Done: "}</b> {"when all contributions were merged upstream"}</li>
                    </ol>
                </p>
            </>
        },
        3 => html! {
            <>
                <p>
                    {"Let's get our hands dirty. "}
                    {"I just received a request for an easy, but definitely game-changing feature. "}
                    {"This is a good first task for you!"}
                </p>
                <p>
                    {"Here, let me file a ticket with the main idea real quick."}
                </p>
            </>
        },
        4 => html! {
            <>
                <p>
                    {"Here it is. You will find the ticket with a unique ID in the Backlog. "}
                    {"But note that this is a "}<em>{"stub"}</em>{". "}
                    {"I mostly grabbed some quotes from the e-mail with the idea and wrote the use case story. "}
                    {"Before we start working on it, we need to get more specific, "}
                    {"nail down the requirements and enumerate acceptance criteria. "}
                </p>
                <p>
                    {"I'll let you take care of this. To prepare the task, "}
                    {"move it by dragging and dropping onto the next column, Sprint candidate."}
                </p>
                <div class="onboarding-wrapper">
                    <div class="board-stage board-stage-tutorial board-stage-backlog">
                        <div class="board-stage-header">{"Backlog"}</div>
                        <div class="board-stage-body">
                        </div>
                    </div>
                    <div class="board-stage board-stage-tutorial board-stage-candidate">
                        <div class="board-stage-header">{"Sprint Candidate"}</div>
                        <div class="board-stage-body"/>
                    </div>
                    <div class="board-task board-task-tutorial-4">
                        {"T351"}
                        <span class="board-task-score">{"+5"}</span>
                        <div class="tutorial-drag"></div>
                    </div>
                </div>
            </>
        },
        5 => html! {
            <>
                <p>{"Yup, that looks OK!"}</p>
                <p>
                    {"This would be the part where you delegate someone to work on it, "}
                    {"by "}<em>{"assigning"}</em>{" the task to someone. "}
                    {"However, this might not always be possible due to lack of staff. "}
                    {"So this time, you'll be the one writing some code."}
                </p>
                <p>
                    {"Assign this task to yourself by dragging and dropping onto your avatar. "}
                    {"Then, you will be able to move it to the next stage, In progress."}
                </p>
                <div class="onboarding-wrapper">
                    <div class="board-task board-task-tutorial-5">
                        {"T351"}
                        <span class="board-task-score">{"+5"}</span>
                        <div class="tutorial-drag"></div>
                    </div>
                    <div class="human-outer you">
                        <div class="human-activity"/>
                            <div class="human-head">
                                <div class="human-eye">
                                    <div class="human-eye-pupil" />
                                </div>
                                <div class="human-eye">
                                    <div class="human-eye-pupil" />
                                </div>
                            </div>
                            <div class="human-body"></div>
                        <div class="human-name">{"You"}</div>
                    </div>

                </div>
            </>
        },
        6 => html! {
            <>
                <p>{"Good work!"}</p>

                <p>
                    {"Once done, we can merge these changes or review them first. "}
                    {"It might happen that you or your developers introduce bugs and whatnot. "}
                </p>
                <p>
                    {"Move the task to the next stage, Under review, "}
                    {"and let it stay there for a while. "}
                </p>
                <div class="message-meta">
                  <h4>{"Meta tip:"}</h4>
                  <p>
                    {"If you feel that time is running slowly, "}
                    {"use the speed buttons in the status bar. "}
                    {"You can even pause the game there, "}
                    {"for when you're under pressure!"}
                  </p>
                </div>
            </>
        },
        7 => html! {
            <>
                <p>{"You found a bug!"}</p>

                <p>
                    {"You're most likely to find bugs than not, so don't worry. "}
                    {"There is still time to fix it. Move it back to In Progress and rework on it."}
                </p>
                <p>
                    {"I mean, nothing stops you from delivering the feature with this bug, "}
                    {"but I don't think you should be indifferent to it."}
                </p>
            </>
        },
        8 => html! {
            <>
                <p>
                    {"You can perform as many review iterations as you like. "}
                    {"The more time you review, the higher the chances of finding more bugs!"}
                </p>

                <p>
                    {"Ultimately though, you'll want to bring it upstream. "}
                    {"Move the task to Done when you no longer intend to work on it."}
                </p>
            </>
        },
        9 => html! {
            <>
                <p>
                    {"Each task has a score representing its overall impact on the product. "}
                    {"Merge more of these tasks each month to increase your score!"}
                </p>
                <p>
                    {"In your spare time, you should think of other things to work on the project "}
                    {"and write them down immediately as stubs. "}
                    {"When you're not pressured by deadlines, "}
                    {"this will enable you to continue making value."}
                </p>
            </>
        },
        10 => html! {
            <>
                <p>
                    {"Ah, you stumbled upon a bug while playing around with the software! "}
                    {"Bug tasks are the right kind for that. "}
                    {"They do not yield as many points, but will improve the quality and image towards our clients. "}
                </p>
                <p>
                    {"Better us finding and fixing the bugs before they complain about them!"}
                </p>
            </>
        },
        11 => html! {
            <>
                <p>
                    {"I will work alongside you as a developer for the rest of the month. "}
                    {"You can either give me tasks to code for, or let me review your own code."}
                </p>
                <p>
                    {"Peer review is generally better: since the writer of the code is a bit biased, "}
                    {"it will be easier for other developers to discover certain bugs. "}
                    {"You might not always have the opportunity to do peer review on each and every task, "}
                    {"but I would strongly recommend it in the future!"}
                </p>
            </>
        },
        12 => html! {
            <>
                <p>
                    {"You have just created a chore task. "}
                    {"Chores to not contribute to your score, but they help to keep the code maintainable. "}
                </p>
                <p>
                    {"Personally, I don't like working with messy code, and neither will your colleagues. "}
                </p>
            </>
        },
        13 => html! {
            <>
                <p>
                    {"I don't have much time left in the team. "}
                    {"Just a few tips before I go:"}
                    <ul>
                        <li>
                            {"You can't work on task specification or task ingestion while also coding or reviewing. "}
                            {"Delegate those to your team of developers as much as you can."}
                        </li>
                        <li>
                            {"Some tasks may have a deadline imposed by the board. "}
                            {"Deliver features on time, otherwise you'll get penalties."}
                        </li>
                        <li>
                            {"QA is still important though. "}
                            {"Working and merging fast on tasks will increase the complexity of the software "}
                            {"and make future tasks harder to work on."}
                        </li>
                    </ul>
                </p>
                <p>
                    {"I hope the onboarding was satisfying to you. I'll see you around. Best wishes! 👋"}
                </p>
            </>
        },
        _ => html! {},
    };

    html! {
        <>
            <div class="tutorial-guy">
            </div>
            <div class="tutorial-body">{text_node}</div>
        </>
    }
}
