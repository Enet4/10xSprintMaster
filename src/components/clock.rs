use yew::prelude::*;

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    /// number of ticks since month start
    pub time: u32,
}

pub struct Clock {
    props: Props,
    link: ComponentLink<Self>,
}

pub struct Msg;

impl Component for Clock {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Clock { props, link }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
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
        let progress = self.props.time as f32 / crate::state::TICKS_PER_MONTH as f32;
        super::progress_bar("clock-outer", "clock-inner", progress)
    }
}
