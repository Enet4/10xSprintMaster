//! A nice abstraction for Drag and Drop data transfer API
//!
//! <https://developer.mozilla.org/en-US/docs/Web/API/HTML_Drag_and_Drop_API>
//!

pub mod payload;

use js_sys::{Array, Function, Object, Reflect};
use serde::{de::DeserializeOwned, Serialize};
use serde_qs;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};

/// A data transfer object
/// that contains the data being transferred to a drop zone.
#[derive(Debug, Clone)]
pub struct DataTransfer {
    value: JsValue,
}

impl DataTransfer {
    pub fn from_event(ev: &impl AsRef<Object>) -> DataTransfer {
        let ev: &Object = ev.as_ref();
        DataTransfer {
            value: Reflect::get(&ev, &"dataTransfer".into()).unwrap_throw(),
        }
    }

    pub fn set_data<T>(&self, content: &T)
    where
        T: Serialize,
    {
        let payload = serde_qs::to_string(content)
            .map_err(|e| JsValue::from_str(&e.to_string()))
            .expect_throw("could not serialize data");
        self.set_text_data("application/x-www-form-urlencoded", &payload)
    }

    pub fn get_data<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        let str_data = self.get_text_data("application/x-www-form-urlencoded");
        serde_qs::from_str(&str_data).expect_throw("could not deserialize data")
    }

    pub fn set_text_data(&self, mime_type: &str, content: &str) {
        let fn_set_data: JsValue = Reflect::get(&self.value, &"setData".into()).unwrap_throw();
        let fn_set_data: Function = fn_set_data.dyn_into().unwrap_throw();
        let args = Array::new();
        args.set(0, JsValue::from_str(&mime_type));
        args.set(1, JsValue::from_str(&content));

        Reflect::apply(&fn_set_data, &self.value, &args)
            .expect_throw("could not set data transfer");
    }

    pub fn get_text_data(&self, mime_type: &str) -> String {
        let fn_get_data: JsValue = Reflect::get(&self.value, &"getData".into()).unwrap_throw();
        let fn_get_data: Function = fn_get_data.dyn_into().unwrap_throw();
        let args = Array::new();
        args.set(0, JsValue::from_str(&mime_type));

        Reflect::apply(&fn_get_data, &self.value, &args)
            .expect_throw("could not get data transfer")
            .as_string()
            .unwrap_throw()
    }

    /// Set the intended drag effect
    pub fn set_drop_effect(&self, effect: DragEffect) {
        let obj: &Object = self.value.dyn_ref().unwrap_throw();

        Reflect::set(&obj, &"dropEffect".into(), &effect.to_js_value())
            .expect_throw("could not set drop effect");
    }

    /// Retrieve the current drag effect
    pub fn drop_effect(&self) -> DragEffect {
        let obj: &Object = self.value.dyn_ref().unwrap_throw();

        let value =
            Reflect::get(&obj, &"dropEffect".into()).expect_throw("could not get drop effect");

        DragEffect::try_from(value).expect_throw("could not get drop effect")
    }
}

/// A drag and drop effect from the Drag and Drop Web API.
///
/// See [`dropEffect` on MDN](https://developer.mozilla.org/en-US/docs/Web/API/DataTransfer/dropEffect)
/// for more information.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum DragEffect {
    /// indicates that the dragged data
    /// will be copied from its present location to the drop location
    Copy,
    /// indicates that the dragged data
    /// will be moved from its present location to the drop location
    Move,
    /// indicates that some form of relationship or connection
    /// will be created between the source and drop locations
    Link,
}

impl DragEffect {
    /// convert to its standard string representation
    pub fn to_name(self) -> &'static str {
        match self {
            DragEffect::Copy => "copy",
            DragEffect::Move => "move",
            DragEffect::Link => "link",
        }
    }

    /// convert a `JsValue` of its standard string representation
    pub fn to_js_value(self) -> JsValue {
        JsValue::from_str(self.to_name())
    }
}

impl TryFrom<JsValue> for DragEffect {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        let value = value
            .as_string()
            .ok_or_else(|| JsValue::from_str("could not convert to string"))?;

        let name = value.as_str();
        match name {
            "copy" => Ok(DragEffect::Copy),
            "move" => Ok(DragEffect::Move),
            "link" => Ok(DragEffect::Link),
            _ => Err(JsValue::from_str(&format!(
                "illegal drag effect name `{}`",
                name
            ))),
        }
    }
}
