# Implementation Plan

## Key Architecture Reference

Before starting any phase, understand these critical patterns:

- **Message flow:** Frontend STOMP SEND → backend publishes to NATS → NATS consumer persists to DB → broadcasts via WebSocket. File attachments must integrate with this flow, NOT bypass it.
- **Backend layering:** Controller (`src/attachments.rs`) → Service (`src/logic/attachments.rs`) → Repository (`src/db/attachments.rs`). Controllers never open transactions or do business logic.
- **Frontend layering:** Components dispatch `ChatAction` variants → reducer updates `ChatState` → components re-render. Components never call HTTP endpoints directly; business logic in `src/logic.rs` or `src/services/` does.
- **Existing patterns to follow:** Look at `src/channels.rs` (controller), `src/db/channels.rs` (repository), `src/logic/ws_logic.rs` (service/logic) for backend. Look at `frontend/src/state.rs` and `frontend/src/logic.rs` for frontend.
- **Test placement:** Backend tests go in `tests/` directory as separate files (e.g., `tests/attachment_tests.rs`). Frontend unit tests go in separate test files (not inline `mod tests`). E2E tests go in `e2e/tests/`.

---

## Phase 1: Database Schema and Repository Layer

### 1.1 Create database migration for `attachments` table
- [x] Task: Create a new SQL migration file in `migrations/` following the existing naming convention (e.g., `migrations/20260416000000_add_attachments.sql`). The table must have these columns:
  - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
  - `message_id UUID NULL REFERENCES messages(id)` — NULL when uploaded but not yet sent with a message
  - `original_filename TEXT NOT NULL` — the user's original filename, sanitized
  - `stored_filename TEXT NOT NULL` — UUID-based filename on disk (e.g., `{uuid}.{extension}`)
  - `file_size BIGINT NOT NULL` — size in bytes
  - `mime_type TEXT NOT NULL` — validated MIME type
  - `uploaded_by UUID NOT NULL REFERENCES users(id)`
  - `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
  - Add an index on `message_id` for efficient lookups: `CREATE INDEX idx_attachments_message_id ON attachments(message_id)`

### 1.2 Add attachment configuration to backend config
- [x] Task: In `src/config.rs`, add two new config fields: `attachment_storage_dir: String` (path to the directory where files are saved on disk) and `max_attachment_size_bytes: u64` (default 50 * 1024 * 1024 = 52428800). Load these from environment variables or the existing `config.json`. Make sure the storage directory is created on startup if it doesn't exist.

### 1.3 Define the `AttachmentMeta` struct
- [x] Task: Create a shared struct that can be used across layers. Place it in `src/messages.rs` alongside the existing `ChatMessage` struct (this file already holds shared message types). The struct:
  ```rust
  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
  pub struct AttachmentMeta {
      pub id: Uuid,
      pub original_filename: String,
      pub file_size: i64,
      pub mime_type: String,
  }
  ```
  This is the **wire format** — it does NOT include `stored_filename` or `uploaded_by` (those are internal). This struct will be included in `ChatMessage` later.

### 1.4 Write repository layer tests and implementation
- [x] Task: Create `tests/attachment_db_tests.rs`. Write failing tests for:
  - `insert_attachment` — inserts a row with `message_id = NULL`, returns the inserted row.
  - `get_attachment_by_id` — retrieves a single attachment by its UUID.
  - `get_attachments_by_message_id` — retrieves all attachments for a given message_id.
  - `link_attachments_to_message` — given a list of attachment UUIDs and a message UUID, UPDATE's the `message_id` column on all matching rows. Must only link attachments where `uploaded_by` matches the requesting user (security check).
  - Tests need a real database (follow existing patterns in `tests/db_tests.rs` for setup).
- [x] Task: Create `src/db/attachments.rs` and implement the four repository functions to make the tests pass. Register it in `src/db/mod.rs`. Each function takes `&mut Transaction<'_, Postgres>` as its first argument (following the existing repository pattern in `src/db/messages.rs`). The `link_attachments_to_message` function must include a `WHERE uploaded_by = $3` clause to prevent users from claiming other users' uploads.

### 1.5 Extend `ChatMessage` with attachments field
- [x] Task: Add `#[serde(default)] pub attachments: Vec<AttachmentMeta>` to `ChatMessage` in `src/messages.rs`. The `#[serde(default)]` ensures backward compatibility — existing messages without attachments will deserialize with an empty vec.
- [x] Task: Add the same field to the frontend's `ChatMessage` in `frontend/src/types.rs`: `#[serde(default)] pub attachments: Vec<AttachmentMeta>`. Also add the `AttachmentMeta` struct to `frontend/src/types.rs` (duplicated from backend — this is intentional since frontend and backend are separate crates).
- [x] Task: Update the `get_messages` function in `src/db/messages.rs` to LEFT JOIN the `attachments` table and populate the `attachments` Vec on each `ChatMessage`. This is the most complex query change. The current query at lines 66-140 builds `ChatMessage` structs from rows — you need to either:
  - (Preferred) Do a second query per batch: after fetching messages, do `SELECT * FROM attachments WHERE message_id = ANY($1)` with the message IDs, then group by message_id and attach to the right ChatMessage. This avoids complex JOIN/grouping logic.
  - Or use a LEFT JOIN with aggregation.
  Choose whichever is simpler. Update existing message query tests to verify attachments are included.

### 1.6 Conductor - User Manual Verification 'Phase 1'
- [x] Task: Conductor - User Manual Verification 'Phase 1: Database Schema and Repository Layer' (Protocol in workflow.md). Verify: migration runs cleanly, all repository tests pass, `ChatMessage` serialization with attachments works (write a small test that serializes/deserializes a `ChatMessage` with and without attachments).

---

## Phase 2: Backend Upload/Download API and NATS Integration

### 2.1 Implement attachment service layer
- [ ] Task: Create `src/logic/attachments.rs` and register it in `src/logic/mod.rs`. Implement these service functions:
  - `upload_attachment(pool: &PgPool, config: &Config, user_id: Uuid, filename: String, mime_type: String, data: Bytes) -> Result<AttachmentMeta, AppError>`:
    1. Validate `data.len()` against `config.max_attachment_size_bytes`. Return error if exceeded.
    2. Validate `mime_type` against an allowlist of permitted MIME types. The allowlist should include at minimum: `image/jpeg`, `image/png`, `image/gif`, `image/webp`, `video/mp4`, `video/webm`, `audio/mpeg`, `audio/wav`, `audio/ogg`, `application/pdf`, `application/octet-stream` (generic fallback for unknown binary files), `text/plain`.
    3. Sanitize `filename`: strip any path components (take only the last segment after `/` or `\`), limit to 255 chars.
    4. Generate a UUID for `stored_filename` with the original file extension (e.g., `{uuid}.png`).
    5. Open a DB transaction, call `db::attachments::insert_attachment(...)`, commit.
    6. Write `data` to `{config.attachment_storage_dir}/{stored_filename}`.
    7. If the disk write fails, the DB row is already committed — this is acceptable (the row has `message_id = NULL` and will be orphaned). Do NOT try to do disk writes inside the transaction.
    8. Return `AttachmentMeta` (the wire-format struct, not internal details).
  - `get_attachment_for_download(pool: &PgPool, config: &Config, attachment_id: Uuid) -> Result<(AttachmentMeta, PathBuf), AppError>`:
    1. Open transaction, call `db::attachments::get_attachment_by_id(...)`.
    2. Return the metadata and the full disk path (`{config.attachment_storage_dir}/{stored_filename}`).
    3. If not found, return a 404 error.

### 2.2 Implement attachment controller (Axum routes)
- [ ] Task: Create `src/attachments.rs` with an Axum router function (follow the pattern in `src/channels.rs`):
  ```rust
  pub fn router() -> Router<CombinedState> {
      Router::new()
          .route("/", post(upload_attachment))
          .route("/:id", get(download_attachment))
  }
  ```
  - `upload_attachment` handler: Extract `UserSession` (authentication), extract `Multipart` form data, read the file field, call the service layer's `upload_attachment`, return JSON with `AttachmentMeta`.
  - `download_attachment` handler: Extract `UserSession` (authentication), extract path param `id: Uuid`, call service layer's `get_attachment_for_download`, return the file bytes with appropriate `Content-Type` and `Content-Disposition` headers.
- [x] Task: Register the new router in `src/router.rs` by adding `.nest("/api/attachments", attachments::router())` — place it next to the existing `/api/auth` and `/api/channels` nests. Add `use crate::attachments;` to the imports.

### 2.3 Write API endpoint tests
- [ ] Task: Create `tests/attachment_api_tests.rs`. Write tests for:
  - Upload a valid file → returns 200 with `AttachmentMeta` containing a valid UUID, original filename, size, MIME type.
  - Upload a file exceeding 50MB → returns 400/413 error.
  - Upload with an invalid MIME type → returns 400 error.
  - Download an existing attachment by ID → returns file bytes with correct Content-Type.
  - Download a non-existent attachment → returns 404.
  - Upload and download without authentication → returns 401.
  - Follow existing test patterns in `tests/api_tests.rs` and `tests/channel_messages_api_tests.rs` for test setup (spinning up the app, creating test users, etc.).

### 2.4 Update NATS consumer to link attachments to messages
- [ ] Task: This is a critical integration point. In `src/logic/nats.rs`, the `handle_chat_message` function (line 103) processes incoming chat messages from NATS. Currently it deserializes `ChannelEntry::Message(ChatMessage)` and persists it. After persisting the message:
  1. Check if `chat_msg.attachments` is non-empty.
  2. If so, extract the attachment IDs from `chat_msg.attachments`.
  3. Open a DB transaction and call `db::attachments::link_attachments_to_message(tx, attachment_ids, message_id, user_id)` to UPDATE the attachment rows' `message_id` column.
  4. This ensures the two-step flow works: upload (REST) creates orphan rows → send message (STOMP/NATS) links them.
  - **Important:** The `link_attachments_to_message` function must verify `uploaded_by = user_id` to prevent a user from claiming someone else's uploaded attachments.

### 2.5 Update STOMP/WebSocket logic to carry attachment IDs
- [ ] Task: In `src/logic/ws_logic.rs`, the `process_and_publish_message` function (line 270) builds a `ChatMessage` and publishes it to NATS. Currently the function signature takes `body: String` (the message text). The message text coming from the frontend STOMP SEND frame will now be a JSON object like `{"message": "hello", "attachment_ids": ["uuid1", "uuid2"]}` instead of a plain string. Update this function to:
  1. Parse the body as JSON to extract the text message and optional `attachment_ids`.
  2. If `attachment_ids` are present, look up their `AttachmentMeta` from the DB (read-only query) so the full metadata can be included in the `ChatMessage`.
  3. Set `chat_msg.attachments` to the resolved `Vec<AttachmentMeta>`.
  4. The message then flows through NATS with attachment metadata included, so all consumers and WebSocket clients receive the full picture.
  - **Note:** If no `attachment_ids` are present (backwards compatibility), treat `body` as plain text message content as before.
  - Update `build_chat_message` to accept an optional `Vec<AttachmentMeta>` parameter.

### 2.6 Docker volume configuration for shared file storage
- [ ] Task: In `docker-compose.yml` and `docker-compose.e2e.yml`, add a named volume (e.g., `attachment-data`) and mount it to the configured `attachment_storage_dir` path on all `app` service replicas. This ensures that in multi-replica deployments (the E2E environment runs 2 replicas), any replica can serve a file uploaded to any other replica. Example:
  ```yaml
  volumes:
    attachment-data:
  services:
    app:
      volumes:
        - attachment-data:/app/data/attachments
  ```

### 2.7 Conductor - User Manual Verification 'Phase 2'
- [x] Task: Conductor - User Manual Verification 'Phase 2: Backend Upload/Download API and NATS Integration' (Protocol in workflow.md). Verify: upload a file via curl/Postman, download it back, send a message with attachment_ids via STOMP, confirm the NATS consumer links the attachment to the message in the DB. All API tests pass. No compiler warnings.

---

## Phase 3: Frontend State, Components, and Upload Integration

### 3.1 Add AttachmentMeta type and update ChatMessage on frontend
- [x] Task: If not already done in Phase 1.5, ensure `frontend/src/types.rs` has the `AttachmentMeta` struct and `ChatMessage` includes `#[serde(default)] pub attachments: Vec<AttachmentMeta>`. Verify that existing message deserialization still works by running the frontend build (`trunk build` or equivalent).

### 3.2 Add attachment state and actions to ChatState
- [x] Task: In `frontend/src/logic.rs`, add new `ChatAction` variants:
  - `AddPendingAttachment(AttachmentMeta)` — adds an uploaded attachment to a pending list for the currently composed message.
  - `RemovePendingAttachment(Uuid)` — removes a pending attachment by ID (user cancelled before sending).
  - `ClearPendingAttachments` — clears all pending attachments after message is sent.
  - `SetAttachmentError(Option<String>)` — sets/clears an error message (e.g., "File exceeds 50MB limit").
- [x] Task: In `ChatState` (defined in `frontend/src/logic.rs`), add fields:
  - `pub pending_attachments: Vec<AttachmentMeta>` — attachments uploaded for the message being composed.
  - `pub attachment_error: Option<String>` — error message to display.
- [x] Task: Implement the reducer arms for the new actions in `frontend/src/state.rs` (in the `Reducible` impl). Follow the existing pattern of delegating to methods on `ChatState`.
- [x] Task: Write unit tests for the new reducer logic in a test file. Test that dispatching `AddPendingAttachment` adds to the list, `RemovePendingAttachment` removes by ID, `ClearPendingAttachments` empties the list, etc.

### 3.3 Implement attachment upload service
- [x] Task: Create `frontend/src/services/attachment.rs` and register in `frontend/src/services/mod.rs`. This module handles the HTTP upload call:
  - `pub async fn upload_file(file: web_sys::File) -> Result<AttachmentMeta, String>`:
    1. Check `file.size()` against 50MB limit. Return error string if exceeded (this is the client-side pre-check).
    2. Read the file into bytes using `gloo::file::FileReadFuture` or equivalent.
    3. Build a `FormData` with the file and POST to `/api/attachments`. The browser automatically includes cookies for authentication.
    4. Parse the JSON response as `AttachmentMeta` and return it.
  - This function is called from business logic (not from components directly). The calling code dispatches `AddPendingAttachment` on success or `SetAttachmentError` on failure.

### 3.4 Implement attachment button component
- [x] Task: Create `frontend/src/components/attachment_button.rs` and register in `frontend/src/components/mod.rs`. This is a small component that renders a paperclip icon button. When clicked, it opens a hidden `<input type="file">` element. When a file is selected:
  1. Dispatch the upload via the business logic layer (spawn an async task that calls the upload service, then dispatches the appropriate `ChatAction`).
  2. While uploading, optionally show a loading indicator.
  - The component reads `pending_attachments` and `attachment_error` from `ChatStateContext` to display pending files and errors.
  - Use `data-testid="attachment-button"` on the button for E2E testing.

### 3.5 Implement drag-and-drop and paste handlers
- [x] Task: In the chat input area component (likely `frontend/src/components/chat_window.rs`), add:
  - **Drag-and-drop:** Add `ondragover` (prevent default to allow drop) and `ondrop` event handlers on the chat area container. On drop, extract files from the `DataTransfer`, trigger the same upload logic as the button.
  - **Clipboard paste:** Add `onpaste` event handler on the message input. On paste, check `ClipboardEvent.clipboardData().files()`. If files are present, trigger upload. If only text, let the default paste behavior proceed.
  - Use `data-testid="chat-area"` on the drop target for E2E testing.
  - Both handlers must go through the same business logic path as the button (dispatch actions, call upload service).

### 3.6 Wire attachments into message sending
- [x] Task: Update the message send logic. Currently when the user sends a message, a STOMP SEND frame is constructed with the message text as the body. Change this so:
  1. If `pending_attachments` is non-empty, construct a JSON body: `{"message": "<text>", "attachment_ids": ["<uuid1>", ...]}` instead of plain text.
  2. After sending, dispatch `ClearPendingAttachments`.
  3. Find where the STOMP SEND frame is constructed in the frontend (likely in `frontend/src/services/websocket.rs` or `frontend/src/stomp.rs`) and update accordingly.
  - **Backward compatibility:** If there are no attachments, you may still send plain text OR always send JSON. Choose one and make sure the backend (Phase 2.5) handles it consistently.

### 3.7 Implement inline attachment renderers
- [x] Task: Create `frontend/src/components/attachment_renderer.rs`. This component takes a `Vec<AttachmentMeta>` as a prop and renders each attachment inline based on its `mime_type`:
  - `image/*` → `<img>` tag with `src="/api/attachments/{id}"`, reasonable max-width styling.
  - `video/*` → `<video controls>` tag with `<source src="/api/attachments/{id}">`.
  - `audio/*` → `<audio controls>` tag with `<source src="/api/attachments/{id}">`.
  - `application/pdf` → `<iframe>` or `<embed>` with `src="/api/attachments/{id}"` and fixed height.
  - Everything else → a download link: `<a href="/api/attachments/{id}" download>{original_filename}</a>` with a file icon.
  - Use `data-testid="attachment-{id}"` on each rendered attachment for E2E testing.
- [x] Task: Integrate the `AttachmentRenderer` component into the existing message display component (wherever individual `ChatMessage` items are rendered). If `message.attachments` is non-empty, render the `AttachmentRenderer` below the message text.

### 3.8 Display pending attachments in compose area
- [x] Task: In the message compose area (near the input field), render a list of `pending_attachments` from state. Each item shows the filename and a remove button (dispatches `RemovePendingAttachment(id)`). Also display `attachment_error` as a red error message if set. Use `data-testid="pending-attachment-{id}"` and `data-testid="attachment-error"` for E2E testing.

### 3.9 Conductor - User Manual Verification 'Phase 3'
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Frontend State, Components, and Upload Integration' (Protocol in workflow.md). Verify: open the app in a browser, attach a file via button click, see it appear as pending, send the message, see the attachment rendered inline. Test drag-and-drop and paste. Try uploading a file > 50MB and verify the error message. No compiler warnings.

---

## Phase 4: E2E Tests and Polish

### 4.1 Add E2E happy-path tests
- [x] Task: Create `e2e/tests/attachments.spec.ts`. Write Playwright E2E tests for:
  - Upload an image via the attachment button → send message → image appears inline in the chat for both sender and receiver (test cross-client delivery).
  - Upload a non-image file (e.g., `.txt`) → send message → download link appears.
  - Try to upload a file > 50MB → error message appears, no upload happens.
  - Use `data-testid` selectors exclusively (never CSS classes or DOM structure).
  - Follow patterns in existing E2E tests in `e2e/tests/`.
  - Run with `cd e2e && npx playwright test --reporter=list`.
  - The Docker stack must be running with the shared volume from Phase 2.6.

### 4.2 Verify multi-replica delivery
- [x] Task: The E2E environment runs 2 app replicas behind nginx. Verify that a file uploaded to replica A can be downloaded from replica B (the shared volume from Phase 2.6 makes this work). This should be implicitly tested by the E2E tests (sender and receiver may hit different replicas), but verify explicitly if possible.

### 4.3 Final cleanup and compilation check
- [x] Task: Run `cargo clippy` and `cargo build` on the backend, `trunk build` (or equivalent) on the frontend. Fix any warnings — the project has a zero-warnings policy. Verify all unit tests pass (`cargo test`). Verify E2E tests pass.

### 4.4 Conductor - User Manual Verification 'Phase 4'
- [x] Task: Conductor - User Manual Verification 'Phase 4: E2E Tests and Polish' (Protocol in workflow.md). Full end-to-end walkthrough: register two users, one sends a message with an image attachment, the other sees it rendered inline. Download works. Error cases handled.
