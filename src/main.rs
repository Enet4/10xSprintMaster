#![allow(dead_code)]
use state::WorldState;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlAudioElement;
use yew::prelude::*;

mod audio;
mod components;
mod services;
mod state;

pub mod data_transfer;
pub mod event_bus;

use components::game::Game;

use crate::audio::play_zipclick;
use crate::components::bug;
use crate::components::game::GameStateOrigin;
use crate::components::modal::Modal;

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    NewProduct,
    NewGame,
    ContinueGame,
    ProductNameInput(String),
    ToggleOnboarding,
    Nothing,
}

#[derive(Debug)]
struct App {
    // `ComponentLink` is like a reference to a component.
    // It can be used to send messages to the component
    link: ComponentLink<Self>,

    /// Our local version of state.
    state: AppState,

    /// Whether we have a saved game.
    has_save: bool,

    /// The product name input value.
    product_name: String,

    /// Whether onboarding checkbox is checked.
    onboarding: bool,

    click_audio: HtmlAudioElement,
}

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    /// The player is in the main menu
    MainMenu,
    /// The player is choosing a product name
    NewProduct,
    /// A game is ongoing
    Game(GameStateOrigin),
}

impl Default for AppState {
    fn default() -> Self {
        AppState::MainMenu
    }
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let has_save = WorldState::has_save_in_storage().unwrap_or(false);

        // pre-load sound assets
        let click_audio = HtmlAudioElement::new_with_src(audio::ZIPCLICK).unwrap_throw();
        click_audio.set_cross_origin(Some("anonymous"));

        Self {
            link,
            //state: AppState::MainMenu,
            state: AppState::default(),
            has_save,
            product_name: String::new(),
            onboarding: true,
            click_audio,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Nothing => false,
            Msg::NewProduct => {
                self.state = AppState::NewProduct;
                true
            }
            Msg::ContinueGame => {
                self.state = AppState::Game(GameStateOrigin::Continue);
                true
            }
            Msg::ProductNameInput(name) => {
                self.product_name = name;
                true
            }
            Msg::ToggleOnboarding => {
                self.onboarding = !self.onboarding;
                true
            }
            Msg::NewGame => {
                self.state = AppState::Game(GameStateOrigin::New {
                    project_name: self.product_name.clone(),
                    onboarding: self.onboarding,
                });
                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        // return always false because component has no properties
        false
    }

    fn view(&self) -> Html {
        match &self.state {
            AppState::MainMenu => {
                let newgame_handler = self.link.callback(move |_| {
                    play_zipclick();

                    Msg::NewProduct
                });
                let continuegame_handler = self.link.callback(move |_| {
                    play_zipclick();

                    Msg::ContinueGame
                });

                html! {
                    <>
                    <div class="main-menu-back" />
                    <div class="main-menu">
                        <h1>{ "10x Sprint Master" }</h1>
                        <div class="main-menu-prompt">
                            <button class="main-menu-button" onclick=newgame_handler>{"New Game"}</button>
                            {
                                if self.has_save {
                                    html! {
                                        <button class="main-menu-button" onclick=continuegame_handler>{"Continue Game"}</button>
                                    }
                                } else {
                                    html! {}
                                }
                            }
                        </div>
                        <div class="main-menu-bug">
                            {bug()}
                        </div>
                        <footer><a href="https://github.com/Enet4/10xSprintMaster">{"On GitHub"}</a></footer>
                    </div>
                    </>
                }
            }
            AppState::NewProduct => {
                let product_name = self.product_name.clone();
                let input_handler = self
                    .link
                    .callback(move |ev: InputData| Msg::ProductNameInput(ev.value));
                let check_handler = self.link.callback(move |_| Msg::ToggleOnboarding);
                let click_audio = self.click_audio.clone();
                let ok_handler = self.link.callback(move |_| {
                    let _ = click_audio.play().unwrap_throw();

                    Msg::NewGame
                });
                let submit_handler = self.link.callback(move |ev: FocusEvent| {
                    // prevent form submission
                    ev.prevent_default();
                    if product_name.is_empty() {
                        Msg::Nothing
                    } else {
                        Msg::NewGame
                    }
                });

                html! {
                    <Modal title="New Game">
                        <form onsubmit=submit_handler>
                            <p>{ "Enter the name of your product:" }</p>
                            <input type="text" class="product-name-input" maxlength=44 placeholder="Product name" oninput=input_handler />

                            <p>
                            <span>
                                <input type="checkbox" class="onboarding" checked=self.onboarding onclick=check_handler /> {"Start with onboarding month (tutorial)"}
                            </span>
                            </p>
                            <p><button onclick=ok_handler disabled={self.product_name.is_empty()}>{"OK"}</button></p>
                        </form>
                    </Modal>
                }
            }
            AppState::Game(origin) => html! {
                <Game state_from={origin.clone()} />
            },
        }
    }
}

fn main() {
    yew::start_app::<App>();
}
