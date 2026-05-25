use yew::prelude::*;
use crate::types::AttachmentMeta;
use crate::state::ChatStateContext;
use crate::logic::ChatAction;
use web_sys::MouseEvent;

#[derive(Properties, PartialEq)]
pub struct AttachmentRendererProps {
    pub attachments: Vec<AttachmentMeta>,
}

#[function_component(AttachmentRenderer)]
pub fn attachment_renderer(props: &AttachmentRendererProps) -> Html {
    let ctx = use_context::<ChatStateContext>().expect("No ChatStateContext found");

    if props.attachments.is_empty() {
        return html! {};
    }

    html! {
        <div class="attachment-renderer" style="display: flex; flex-direction: column; gap: 8px; margin-top: 8px;">
            { for props.attachments.iter().map(|att| {
                let id_str = att.id.to_string();
                let url = format!("/api/attachments/{}", id_str);
                let mime = &att.mime_type;

                let on_img_click = {
                    let url_clone = url.clone();
                    let alt_clone = att.original_filename.clone();
                    let dispatch = ctx.dispatch.clone();
                    Callback::from(move |_e: MouseEvent| {
                        dispatch.emit(ChatAction::OpenImageLightbox {
                            url: url_clone.clone(),
                            alt: alt_clone.clone(),
                        });
                    })
                };
                let img_testid = format!("attachment-img-{}", id_str);

                html! {
                    <div class="attachment-item" data-testid={format!("attachment-{}", id_str)} style="border: 1px solid #ddd; padding: 8px; border-radius: 4px; max-width: 300px;">
                        if mime.starts_with("image/") {
                            <img src={url.clone()}
                                 alt={att.original_filename.clone()}
                                 data-testid={img_testid}
                                 onclick={on_img_click}
                                 style="max-width: 100%; max-height: 200px; display: block;" />
                        } else if mime.starts_with("video/") {
                            <video controls=true src={url} style="max-width: 100%; max-height: 200px; display: block;" />
                        } else if mime.starts_with("audio/") {
                            <audio controls=true src={url} style="width: 100%; display: block;" />
                        } else if mime == "application/pdf" {
                            <embed src={url} type="application/pdf" style="width: 100%; height: 200px; display: block;" />
                        } else {
                            <a href={url} download={att.original_filename.clone()} style="display: flex; align-items: center; gap: 4px; text-decoration: none; color: #007bff;">
                                <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
                                    <polyline points="13 2 13 9 20 9" />
                                </svg>
                                <span>{ &att.original_filename }</span>
                                <span style="color: #666; font-size: 0.8em;">{ format!("({} bytes)", att.file_size) }</span>
                            </a>
                        }
                    </div>
                }
            }) }
        </div>
    }
}
