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
    pub on_open_join_modal: Callback<bool>, // true if create mode
}

#[function_component(Sidebar)]
pub fn sidebar(props: &SidebarProps) -> Html {
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

            <div class="sidebar-actions">
                <button class="browse-button" onclick={let on_open = props.on_open_join_modal.clone(); move |_| on_open.emit(false)}>
                    { "Browse Channels" }
                </button>
                <button class="create-button" onclick={let on_open = props.on_open_join_modal.clone(); move |_| on_open.emit(true)}>
                    { "+" }
                </button>
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
        </div>
    }
}
