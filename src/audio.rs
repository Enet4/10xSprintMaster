use wasm_bindgen::{JsValue, UnwrapThrowExt};
use web_sys::HtmlAudioElement;

use crate::state::try_local_storage;

pub static ZIPCLICK: &str = "assets/audio/zipclick.wav";
pub static ENDOFMONTH: &str = "assets/audio/webuiltthis.ogg";

pub fn play_zipclick() {
    play(ZIPCLICK);
}

pub fn play_endofmonth() {
    play(ENDOFMONTH);
}

pub fn play(file_path: &str) {
    match is_enabled() {
        Ok(true) => {
            let click_audio = HtmlAudioElement::new_with_src(file_path).unwrap_throw();
            click_audio.set_cross_origin(Some("anonymous"));
            let _ = click_audio.play();
        }
        Ok(false) => {},
        Err(e) => {
            gloo_console::error!("Error playing audio:", e);
        }
    }
}

pub fn is_enabled() -> Result<bool, JsValue> {
    let local_storage = try_local_storage()?;
    let out = local_storage.get("audio")?;

    if let Some(enabled) = out {
        Ok(enabled == "true")
    } else {
        local_storage.set("audio", "true")?;
        Ok(true)
    }
}

pub fn set_audio(enabled: bool) -> Result<(), JsValue> {
    let local_storage = try_local_storage()?;
    local_storage.set("audio", if enabled { "true" } else { "false" })?;
    Ok(())
}
