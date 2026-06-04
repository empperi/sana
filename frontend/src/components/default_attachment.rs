use yew::prelude::*;
use crate::types::AttachmentMeta;
use crate::attachment_handlers::format_file_size;

#[derive(Properties, PartialEq)]
pub struct DefaultAttachmentProps {
    pub attachment: AttachmentMeta,
}

#[function_component(DefaultAttachment)]
pub fn default_attachment(props: &DefaultAttachmentProps) -> Html {
    let att = &props.attachment;
    let id_str = att.id.to_string();
    let url = format!("/api/attachments/{}", id_str);
    let size_str = format_file_size(att.file_size);

    html! {
        <div class="default-attachment">
            <div class="attachment-icon">
                <svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
                    <polyline points="13 2 13 9 20 9" />
                </svg>
            </div>
            <div class="attachment-meta">
                <span class="attachment-filename" title={att.original_filename.clone()} data-testid={format!("attachment-filename-{}", id_str)}>
                    { &att.original_filename }
                </span>
                <span class="attachment-info">
                    { format!("{} • {}", size_str, att.mime_type) }
                </span>
            </div>
            <a class="attachment-download" 
               href={url} 
               download={att.original_filename.clone()} 
               role="button"
               data-testid={format!("attachment-download-{}", id_str)}>
                { "Download" }
            </a>
        </div>
    }
}
