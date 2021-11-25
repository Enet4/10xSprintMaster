use yew::prelude::*;

pub mod board;
pub mod clock;
pub mod game;
pub mod human;
pub mod menu;
pub mod messages;
pub mod modal;
pub mod pause;
pub mod stage;
pub mod task;

pub fn progress_bar(outer_class: &'static str, inner_class: &'static str, progress: f32) -> Html {
    let style = format!("width: {}%;", progress * 100.);
    html! {
        <div class=outer_class>
            <div class=inner_class style=style />
        </div>
    }
}

pub fn bug() -> Html {
    html! {
        <div class="bug-outer">
            <div class="bug-body">
                <div class="bug-eye" />
                <div class="bug-eye" />
            </div>
            <div class="bug-arms"></div>
            <div class="bug-arms bug-arms-extra"></div>
        </div>
    }
}
