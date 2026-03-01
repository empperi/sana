use yew::prelude::*;
use std::collections::HashSet;
use crate::services::websocket::ConnectionStatus;

#[derive(Properties, PartialEq)]
pub struct SidebarProps {
    pub channels: Vec<String>,
    pub current_channel: String,
    pub unread_channels: HashSet<String>,
    pub connection_status: ConnectionStatus,
    pub on_switch_channel: Callback<String>,
    pub on_create_channel: Callback<String>,
}

#[function_component(Sidebar)]
pub fn sidebar(props: &SidebarProps) -> Html {
    let new_channel_input = use_state(String::new);

    let status_class = match props.connection_status {
        ConnectionStatus::Connected => "status-connected",
        ConnectionStatus::Disconnected => "status-disconnected",
        ConnectionStatus::Reconnecting => "status-reconnecting",
    };

    let status_text = match props.connection_status {
        ConnectionStatus::Connected => "Connected",
        ConnectionStatus::Disconnected => "Offline",
        ConnectionStatus::Reconnecting => "Connecting...",
    };

    let on_channel_input = {
        let new_channel_input = new_channel_input.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_channel_input.set(input.value());
        })
    };

    let create_channel = {
        let new_channel_input = new_channel_input.clone();
        let on_create_channel = props.on_create_channel.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let name = (*new_channel_input).clone();
            if !name.is_empty() {
                on_create_channel.emit(name);
                new_channel_input.set(String::new());
            }
        })
    };

    html! {
        <div class="sidebar">
            <div class="sidebar-header">
                <img src="/assets/Sana_logo.webp" alt="Sana Logo" class="logo" />
                <div class="header-content">
                    <h2>{ "Sana" }</h2>
                    <div class={classes!("connection-status", status_class)}>
                        <span class="indicator"></span>
                        { status_text }
                    </div>
                </div>
            </div>
            <ul class="channel-list">
                { for props.channels.iter().map(|channel| {
                    let channel_name = channel.clone();
                    let is_active = props.current_channel == channel_name;
                    let has_unread = props.unread_channels.contains(&channel_name);

                    let li_classes = classes!(
                        if is_active { Some("active") } else { None },
                        if has_unread { Some("unread") } else { None }
                    );

                    let onclick = {
                        let on_switch_channel = props.on_switch_channel.clone();
                        let name = channel_name.clone();
                        Callback::from(move |_| on_switch_channel.emit(name.clone()))
                    };
                    html! {
                        <li key={channel_name.clone()} class={li_classes} {onclick}>
                            { format!("# {}", channel_name) }
                        </li>
                    }
                }) }
            </ul>
            <form class="new-channel-form" onsubmit={create_channel}>
                <input
                    type="text"
                    placeholder="New channel..."
                    value={(*new_channel_input).clone()}
                    oninput={on_channel_input}
                />
                <button type="submit">{ "+" }</button>
            </form>
        </div>
    }
}
