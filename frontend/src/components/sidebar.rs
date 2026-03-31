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
    pub is_mobile_open: bool,
    pub on_close_sidebar: Callback<()>,
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

    let sidebar_classes = classes!(
        "sidebar",
        if props.is_mobile_open { Some("mobile-open") } else { None }
    );

    html! {
        <>
            if props.is_mobile_open {
                <div class="sidebar-overlay" onclick={let on_close = props.on_close_sidebar.clone(); move |_| on_close.emit(())}></div>
            }
            <div class={sidebar_classes} data-testid="sidebar">
                <div class="sidebar-header">
                    <img src="/assets/Sana_logo.webp" alt="Sana Logo" class="logo" />
                    <div class="header-content">
                        <h2>{ "Sana" }</h2>
                        <div class={classes!("connection-status", status_class)} data-testid="connection-status">
                            <span class="indicator"></span>
                            { status_text }
                        </div>
                    </div>
                </div>

                <div class="sidebar-actions">
                    <button class="browse-button" data-testid="browse-channels-button" onclick={let on_open = props.on_open_join_modal.clone(); move |_| on_open.emit(false)}>
                        { "Browse Channels" }
                    </button>
                    <button class="create-button" data-testid="open-create-channel-modal" onclick={let on_open = props.on_open_join_modal.clone(); move |_| on_open.emit(true)}>
                        { "+" }
                    </button>
                </div>

                <ul class="channel-list" data-testid="channel-list">
                    { for props.channels.iter().map(|channel| {
                        let channel_name = channel.clone();
                        let is_active = props.current_channel == channel_name;
                        let has_unread = props.unread_channels.contains(&channel_name) && !is_active;

                        let li_classes = classes!(
                            if is_active { Some("active") } else { None },
                            if has_unread { Some("unread") } else { None }
                        );

                        let onclick = {
                            let on_switch_channel = props.on_switch_channel.clone();
                            let on_close_sidebar = props.on_close_sidebar.clone();
                            let name = channel_name.clone();
                            Callback::from(move |_| {
                                on_switch_channel.emit(name.clone());
                                on_close_sidebar.emit(());
                            })
                        };
                        html! {
                            <li key={channel_name.clone()} class={li_classes} {onclick}>
                                { format!("# {}", channel_name) }
                            </li>
                        }
                    }) }
                </ul>
            </div>
        </>
    }
}
