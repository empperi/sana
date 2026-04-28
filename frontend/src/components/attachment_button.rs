use yew::prelude::*;
use web_sys::HtmlInputElement;
use crate::state::ChatStateContext;
use crate::logic::ChatAction;
use crate::services::attachment::upload_file;
use wasm_bindgen_futures::spawn_local;

#[function_component(AttachmentButton)]
pub fn attachment_button() -> Html {
    let ctx = use_context::<ChatStateContext>().expect("No ChatStateContext found");
    let is_uploading = use_state(|| false);
    
    let file_input_ref = use_node_ref();

    let on_button_click = {
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };

    let on_file_change = {
        let is_uploading = is_uploading.clone();
        let dispatch = ctx.dispatch.clone();
        let file_input_ref = file_input_ref.clone();
        
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            
            if let Some(files) = input.files() {
                if files.length() > 0 {
                    if let Some(file) = files.get(0) {
                        let is_uploading = is_uploading.clone();
                        let dispatch = dispatch.clone();
                        
                        is_uploading.set(true);
                        dispatch.emit(ChatAction::SetAttachmentError(None)); // Clear previous errors
                        
                        spawn_local(async move {
                            match upload_file(file).await {
                                Ok(meta) => {
                                    dispatch.emit(ChatAction::AddPendingAttachment(meta));
                                }
                                Err(err_msg) => {
                                    dispatch.emit(ChatAction::SetAttachmentError(Some(err_msg)));
                                }
                            }
                            is_uploading.set(false);
                        });
                    }
                }
            }
            
            // Clear the input so selecting the same file again triggers the event
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
            }
        })
    };

    html! {
        <div class="attachment-button-container">
            <input 
                type="file" 
                ref={file_input_ref}
                onchange={on_file_change} 
                style="display: none" 
                data-testid="file-input"
            />
            <button 
                class={classes!("attachment-btn", if *is_uploading { "uploading" } else { "" })}
                onclick={on_button_click}
                disabled={*is_uploading}
                data-testid="attachment-button"
                title="Attach file"
            >
                if *is_uploading {
                    <span class="spinner" data-testid="upload-spinner">{"⌛"}</span>
                } else {
                    <svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48" />
                    </svg>
                }
            </button>
        </div>
    }
}
