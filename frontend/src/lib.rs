use yew_router::prelude::*;

pub mod components;
pub mod services;
pub mod types;
pub mod stomp;
pub mod logic;
pub mod hooks;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Chat,
    #[at("/login")]
    Login,
    #[at("/register")]
    Register,
    #[not_found]
    #[at("/404")]
    NotFound,
}

pub fn get_api_url(path: &str) -> String {
    path.to_string()
}
