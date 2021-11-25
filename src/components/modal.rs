use yew::prelude::*;

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    #[prop_or_default]
    pub title: String,
    #[prop_or_default]
    pub children: Children,
}

/// View component for a modal box
/// that sits over the screen and can be customized with buttons.
pub struct Modal {
    props: Props,
    link: ComponentLink<Self>,
}

impl Component for Modal {
    type Message = ();
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Modal { props, link }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        false
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        create_modal(&self.props.title, self.props.children.clone())
    }
}

pub fn create_modal(title: &str, children: Children) -> Html {
    html! {
        <>
        <div class="modal-background" />
        <div class="modal">
            <h2>{title}</h2>

            <div class="modal-content">
                {children}
            </div>
        </div>
        </>
    }
}
