use yew::prelude::*;
use crate::types::AttachmentMeta;
use crate::state::ChatStateContext;
use crate::logic::ChatAction;
use web_sys::MouseEvent;

#[derive(Properties, PartialEq)]
pub struct ImageAttachmentProps {
    pub attachment: AttachmentMeta,
}

#[function_component(ImageAttachment)]
pub fn image_attachment(props: &ImageAttachmentProps) -> Html {
    let ctx = use_context::<ChatStateContext>().expect("No ChatStateContext found");
    let att = &props.attachment;
    let id_str = att.id.to_string();
    let url = format!("/api/attachments/{}", id_str);
    let alt = att.original_filename.clone();
    
    let on_click = {
        let url = url.clone();
        let alt = alt.clone();
        let dispatch = ctx.dispatch.clone();
        Callback::from(move |_e: MouseEvent| {
            dispatch.emit(ChatAction::OpenImageLightbox {
                url: url.clone(),
                alt: alt.clone(),
            });
        })
    };

    html! {
        <img src={url}
             alt={alt}
             data-testid={format!("attachment-img-{}", id_str)}
             onclick={on_click}
             style="max-width: 100%; max-height: 200px; display: block;" />
    }
}
