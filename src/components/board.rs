use std::rc::Rc;

use yew::prelude::*;

use super::stage::Stage;

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    pub product_name: Rc<str>,
    pub alert_message: Option<String>,
    #[prop_or_default]
    pub children: ChildrenWithProps<Stage>,
}

pub struct Board {
    props: Props,
    link: ComponentLink<Self>,
}

impl Component for Board {
    type Message = ();
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Board { props, link }
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
        let alert_message = self.props.alert_message
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");

        let alert_classes = if alert_message.is_empty() {
            "alert-message alert-message-hide"
        } else {
            "alert-message alert-message-show"
        };
        
        html! {
            <div class="board">
                <h3>{&self.props.product_name}{" Workboard"}</h3>
                <div class=alert_classes>{alert_message}</div>
                { for self.props.children.iter() }
            </div>
        }
    }
}
