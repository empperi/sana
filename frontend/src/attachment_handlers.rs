use crate::types::AttachmentMeta;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttachmentHandlerKind {
    Image,
    Default,
}

type HandlerMatcher = fn(&str) -> bool;

const HANDLERS: &[(HandlerMatcher, AttachmentHandlerKind)] = &[
    (is_image_mime, AttachmentHandlerKind::Image),
];

fn is_image_mime(mime: &str) -> bool {
    mime.starts_with("image/")
}

pub fn resolve_handler(att: &AttachmentMeta) -> AttachmentHandlerKind {
    HANDLERS.iter()
        .find(|(matcher, _)| matcher(&att.mime_type))
        .map(|(_, kind)| *kind)
        .unwrap_or(AttachmentHandlerKind::Default)
}

pub fn format_file_size(bytes: i64) -> String {
    if bytes <= 0 {
        return "0 B".to_string();
    }

    let original = bytes;
    let bytes = bytes as f64;
    let kb = 1024.0;
    let mb = kb * 1024.0;
    let gb = mb * 1024.0;

    if bytes >= gb {
        return format!("{:.1} GB", bytes / gb);
    }

    if bytes >= mb {
        return format!("{:.1} MB", bytes / mb);
    }

    if bytes >= kb {
        return format!("{:.1} KB", bytes / kb);
    }

    format!("{} B", original)
}
