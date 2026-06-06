use yew::prelude::*;
use crate::types::AttachmentMeta;
use crate::attachment_handlers::{resolve_handler, AttachmentHandlerKind};
use crate::components::image_attachment::ImageAttachment;
use crate::components::default_attachment::DefaultAttachment;

#[derive(Properties, PartialEq)]
pub struct AttachmentProps {
    pub attachment: AttachmentMeta,
}

#[function_component(Attachment)]
pub fn attachment(props: &AttachmentProps) -> Html {
    let handler_kind = resolve_handler(&props.attachment);

    match handler_kind {
        AttachmentHandlerKind::Image => html! {
            <ImageAttachment attachment={props.attachment.clone()} />
        },
        AttachmentHandlerKind::Default => html! {
            <DefaultAttachment attachment={props.attachment.clone()} />
        },
    }
}
