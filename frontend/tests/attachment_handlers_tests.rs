use frontend::attachment_handlers::{resolve_handler, format_file_size, AttachmentHandlerKind};
use frontend::types::AttachmentMeta;

#[test]
fn test_resolve_handler_images() {
    assert_eq!(resolve_handler(&att_with_mime("image/png")), AttachmentHandlerKind::Image);
    assert_eq!(resolve_handler(&att_with_mime("image/jpeg")), AttachmentHandlerKind::Image);
    assert_eq!(resolve_handler(&att_with_mime("image/webp")), AttachmentHandlerKind::Image);
    assert_eq!(resolve_handler(&att_with_mime("image/gif")), AttachmentHandlerKind::Image);
}

#[test]
fn test_resolve_handler_non_images() {
    assert_eq!(resolve_handler(&att_with_mime("application/pdf")), AttachmentHandlerKind::Default);
    assert_eq!(resolve_handler(&att_with_mime("video/mp4")), AttachmentHandlerKind::Default);
    assert_eq!(resolve_handler(&att_with_mime("audio/mpeg")), AttachmentHandlerKind::Default);
    assert_eq!(resolve_handler(&att_with_mime("text/plain")), AttachmentHandlerKind::Default);
}

#[test]
fn test_resolve_handler_unknown_and_empty() {
    assert_eq!(resolve_handler(&att_with_mime("application/octet-stream")), AttachmentHandlerKind::Default);
    assert_eq!(resolve_handler(&att_with_mime("")), AttachmentHandlerKind::Default);
}

#[test]
fn test_format_file_size_bytes() {
    assert_eq!(format_file_size(0), "0 B");
    assert_eq!(format_file_size(512), "512 B");
    assert_eq!(format_file_size(1023), "1023 B");
}

#[test]
fn test_format_file_size_kb() {
    assert_eq!(format_file_size(1024), "1.0 KB");
    assert_eq!(format_file_size(1536), "1.5 KB");
}

#[test]
fn test_format_file_size_mb() {
    assert_eq!(format_file_size(1048576), "1.0 MB");
    assert_eq!(format_file_size(2621440), "2.5 MB");
}

#[test]
fn test_format_file_size_gb() {
    assert_eq!(format_file_size(1073741824), "1.0 GB");
    assert_eq!(format_file_size(1610612736), "1.5 GB");
}

#[test]
fn test_format_file_size_negative() {
    assert_eq!(format_file_size(-100), "0 B");
}

fn att_with_mime(mime: &str) -> AttachmentMeta {
    AttachmentMeta {
        id: uuid::Uuid::new_v4(),
        original_filename: "test.file".to_string(),
        mime_type: mime.to_string(),
        file_size: 100,
    }
}
