use yew::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEvent, MouseEvent};
use crate::logic::ChatAction;
use crate::state::ChatStateContext;

fn close_with_history_pop(
    dispatch: &Callback<ChatAction>,
    history_pushed: &Rc<RefCell<bool>>,
) {
    let was_pushed = *history_pushed.borrow();
    *history_pushed.borrow_mut() = false;

    if was_pushed {
        if let Some(history) = web_sys::window().and_then(|w| w.history().ok()) {
            let _ = history.back();
        }
    }

    dispatch.emit(ChatAction::CloseImageLightbox);
}

fn lock_body_scroll() -> Option<String> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let body = document.body()?;
    let prev = body.style().get_property_value("overflow").ok()?;
    let _ = body.style().set_property("overflow", "hidden");
    Some(prev)
}

fn restore_body_scroll(prev: &Option<String>) {
    if let Some(body) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.body()) {
        match prev {
            Some(v) if !v.is_empty() => {
                let _ = body.style().set_property("overflow", v);
            }
            _ => {
                let _ = body.style().remove_property("overflow");
            }
        }
    }
}

fn restore_focus(previous_focus: &Rc<RefCell<Option<web_sys::Element>>>) {
    if let Some(el) = previous_focus.borrow_mut().take() {
        if let Ok(html_el) = el.dyn_into::<HtmlElement>() {
            let _ = html_el.focus();
        }
    }
}

fn setup_keydown_listener(
    window: &web_sys::Window,
    dispatch: Callback<ChatAction>,
    history_pushed: Rc<RefCell<bool>>,
) -> gloo_events::EventListener {
    gloo_events::EventListener::new(window, "keydown", move |event| {
        let event = event.dyn_ref::<KeyboardEvent>().unwrap();
        if event.key() == "Escape" {
            event.prevent_default();
            close_with_history_pop(&dispatch, &history_pushed);
        }
    })
}

fn setup_popstate_listener(
    window: &web_sys::Window,
    dispatch: Callback<ChatAction>,
    history_pushed: Rc<RefCell<bool>>,
) -> gloo_events::EventListener {
    gloo_events::EventListener::new(window, "popstate", move |_event| {
        *history_pushed.borrow_mut() = false;
        dispatch.emit(ChatAction::CloseImageLightbox);
    })
}

fn handle_cleanup(
    is_open: bool,
    prev_body_overflow: &Option<String>,
    previous_focus: &Rc<RefCell<Option<web_sys::Element>>>,
    history_pushed: &Rc<RefCell<bool>>,
) {
    if is_open {
        restore_body_scroll(prev_body_overflow);
        restore_focus(previous_focus);

        let was_pushed = *history_pushed.borrow();
        if was_pushed {
            if let Some(window) = web_sys::window() {
                if let Ok(history) = window.history() {
                    let _ = history.back();
                }
            }
            *history_pushed.borrow_mut() = false;
        }
    }
}

#[function_component(ImageLightbox)]
pub fn image_lightbox() -> Html {
    let ctx = use_context::<ChatStateContext>().expect("ChatStateContext not found");
    let lightbox = ctx.state.lightbox_image.clone();

    let history_pushed = use_mut_ref(|| false);
    let previous_focus = use_mut_ref(|| None::<web_sys::Element>);
    let close_button_ref = use_node_ref();

    {
        let dispatch = ctx.dispatch.clone();
        let history_pushed = history_pushed.clone();
        let previous_focus = previous_focus.clone();
        let close_button_ref = close_button_ref.clone();
        let is_open = lightbox.is_some();

        use_effect_with(is_open, move |&is_open| {
            let mut keydown_listener: Option<gloo_events::EventListener> = None;
            let mut popstate_listener: Option<gloo_events::EventListener> = None;
            let mut prev_body_overflow: Option<String> = None;

            if is_open {
                let window = web_sys::window().expect("no window");
                let document = window.document().expect("no document");
                
                *previous_focus.borrow_mut() = document.active_element();
                prev_body_overflow = lock_body_scroll();

                let _ = window.history().unwrap().push_state_with_url(&JsValue::NULL, "", Some(""));
                *history_pushed.borrow_mut() = true;

                if let Some(btn) = close_button_ref.cast::<HtmlElement>() {
                    let _ = btn.focus();
                }

                keydown_listener = Some(setup_keydown_listener(&window, dispatch.clone(), history_pushed.clone()));
                popstate_listener = Some(setup_popstate_listener(&window, dispatch.clone(), history_pushed.clone()));
            }

            move || {
                drop(keydown_listener);
                drop(popstate_listener);
                handle_cleanup(is_open, &prev_body_overflow, &previous_focus, &history_pushed);
            }
        });
    }

    let on_close = {
        let dispatch = ctx.dispatch.clone();
        let history_pushed = history_pushed.clone();
        Callback::from(move |_e: MouseEvent| {
            close_with_history_pop(&dispatch, &history_pushed);
        })
    };

    let stop = Callback::from(|e: MouseEvent| e.stop_propagation());

    match lightbox {
        None => html! {},
        Some(img) => html! {
            <div class="image-lightbox-overlay"
                 role="dialog"
                 aria-modal="true"
                 aria-label="Image preview"
                 data-testid="image-lightbox-overlay"
                 onclick={on_close.clone()}>
                <div class="image-lightbox-container" onclick={stop}>
                    <button class="image-lightbox-close"
                            ref={close_button_ref}
                            data-testid="lightbox-close-button"
                            aria-label="Close image preview"
                            onclick={on_close.clone()}>
                        { "×" }
                    </button>
                    <img class="image-lightbox-img"
                         data-testid="lightbox-image"
                         src={img.url}
                         alt={img.alt} />
                </div>
            </div>
        },
    }
}
