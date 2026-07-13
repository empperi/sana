use yew::prelude::*;
use crate::types::AttachmentMeta;
use crate::components::attachment::Attachment;

#[derive(Properties, PartialEq)]
pub struct AttachmentRendererProps {
    pub attachments: Vec<AttachmentMeta>,
}

#[function_component(AttachmentRenderer)]
pub fn attachment_renderer(props: &AttachmentRendererProps) -> Html {
    if props.attachments.is_empty() {
        return html! {};
    }

    html! {
        <div class="attachment-renderer">
            { for props.attachments.iter().map(|att| {
                let id_str = att.id.to_string();
                html! {
                    <div class="attachment-item" data-testid={format!("attachment-{}", id_str)}>
                        <Attachment attachment={att.clone()} />
                    </div>
                }
            }) }
        </div>
    }
}
