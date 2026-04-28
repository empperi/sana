use crate::types::AttachmentMeta;
use gloo_net::http::Request;
use web_sys::{File, FormData};
use wasm_bindgen::JsValue;

const MAX_FILE_SIZE: f64 = 52428800.0; // 50MB in bytes

pub async fn upload_file(file: File) -> Result<AttachmentMeta, String> {
    if file.size() > MAX_FILE_SIZE {
        return Err(format!("File '{}' exceeds 50MB limit.", file.name()));
    }

    let form_data = FormData::new().map_err(|e| format!("Failed to create FormData: {:?}", e))?;
    form_data.append_with_blob_and_filename("file", &file, &file.name())
        .map_err(|e| format!("Failed to append file to form: {:?}", e))?;

    let resp = Request::post("/api/attachments")
        .credentials(web_sys::RequestCredentials::Include)
        .body(JsValue::from(form_data))
        .map_err(|e| format!("Failed to build request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Network error uploading file: {}", e))?;

    if !resp.ok() {
        let err_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Server returned {}: {}", resp.status(), err_text));
    }

    resp.json::<AttachmentMeta>().await.map_err(|e| format!("Failed to parse response: {}", e))
}
