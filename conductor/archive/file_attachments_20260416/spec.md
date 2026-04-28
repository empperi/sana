# File Attachments Specification

## Overview
Implement the capability for users to attach files to chat messages. Files can be attached via drag-and-drop, an attachment button, or clipboard paste. Attached files will be displayed adjacent to the message text. Media files (Images, Videos, Audio, and PDFs) will be rendered inline within the chat for immediate viewing.

## Architecture Context

This project uses a **read-your-own-writes** architecture. All chat messages flow through:
```
Frontend STOMP SEND → Backend publishes to NATS JetStream → NATS consumer picks up → persists to DB → broadcasts to WebSocket clients
```

File attachments must integrate with this flow as follows:
1. **Upload phase (REST):** User uploads a file via `POST /api/attachments`. Backend validates, saves to disk, inserts metadata into DB, and returns an `attachment_id`. The attachment row has `message_id = NULL` at this point (orphaned/pending).
2. **Send phase (STOMP/NATS):** User sends a STOMP SEND frame whose body includes `attachment_ids: [...]`. This flows through NATS as usual.
3. **Consume phase (NATS consumer):** When the NATS consumer persists the message to DB, it also UPDATE's the attachment rows to set `message_id` to the newly persisted message's ID.

This means the REST upload endpoint and the STOMP/NATS message flow are **two separate protocols** that must be coordinated on the frontend via attachment IDs.

### Backend Layering (controller-service-repository)

All backend code must follow the existing layered architecture:
- **Controller** (`src/attachments.rs`): Axum route handlers. Parses multipart form data, extracts `UserSession`, delegates to service.
- **Service** (`src/logic/attachments.rs`): Business logic. Opens DB transactions, validates file size/MIME, calls repository to insert metadata, writes file to disk, commits transaction.
- **Repository** (`src/db/attachments.rs`): Pure SQL functions. `insert_attachment`, `get_attachments_by_message_id`, `link_attachments_to_message`, `get_attachment_by_id`.

Controllers must **never** open DB transactions or write files directly.

### Frontend Architecture (Yew)

All frontend code must follow the existing patterns:
- **Global state** (`ChatState` via `use_reducer`): All attachment state lives here. Components never own attachment state.
- **Actions** (`ChatAction` enum): New variants like `AddPendingAttachment`, `RemovePendingAttachment`, `SetUploadError`.
- **Business logic** (`src/logic.rs`): Upload service calls, attachment-to-message wiring. Components dispatch actions; they never call HTTP endpoints directly.
- **Components**: Pure rendering. Subscribe to state, dispatch actions on user interaction.

## Functional Requirements
- **Attachment Triggers:**
  - Users must be able to drag and drop files onto the chat area to attach them.
  - An attachment button (e.g., a paperclip icon) must be available next to the message input field to open a standard file selection dialog.
  - Users must be able to paste files (e.g., screenshots) directly from their clipboard into the chat input to attach them.
- **Message Association (Two-Step Flow):**
  1. File is uploaded via REST and gets an `attachment_id` back. At this point `message_id` is NULL in the DB.
  2. When the user sends the message, the STOMP SEND frame body includes the `attachment_ids`. The NATS consumer links them upon message persistence.
  - Orphaned attachments (uploaded but never sent) should be handled gracefully — they remain in the DB with `message_id = NULL` and can be cleaned up later (out of scope for initial implementation, but the schema must support it).
- **Inline Rendering:**
  - The following file types must be rendered inline when attached:
    - Images (JPEG, PNG, GIF, WebP)
    - Videos (MP4, WebM)
    - Audio (MP3, WAV, OGG)
    - PDF Documents (using the browser's built-in PDF viewer)
  - Non-renderable files must be displayed as a downloadable link or icon with the file name.
- **Size Limits:**
  - A maximum file size limit of 50 MB per attachment must be enforced on both client and server.
  - The UI must provide clear feedback if a user attempts to upload a file exceeding this limit.
- **Storage Strategy:**
  - File metadata (id, message_id, filename, size, MIME type, storage path, uploaded_by, created_at) will be stored in PostgreSQL.
  - The actual file binary data will be saved to a dedicated directory on the local file system of the backend server.
  - Files should be stored with a UUID-based filename (not the original name) to prevent path traversal and collisions. The original filename is preserved in the DB metadata.
- **File Serving:**
  - `GET /api/attachments/:id` serves the file. This endpoint requires authentication (`UserSession`).
  - The frontend constructs download/inline URLs using this pattern.

## Non-Functional Requirements
- **Storage Management:** The system must efficiently handle writing and reading files from the local file system.
- **Security:**
  - Validate MIME type against an allowlist of permitted types on the server (not just the client-provided Content-Type).
  - Sanitize original filenames (strip path components, limit length).
  - Store files with UUID names to prevent path traversal attacks.
  - The download endpoint must verify the requesting user is authenticated.
- **Multi-Replica Awareness:** File storage is local to the server filesystem. In a multi-replica deployment (e.g., the E2E test environment with 2 replicas), all replicas must share the same storage directory via a Docker volume mount. This must be configured in `docker-compose.yml` and `docker-compose.e2e.yml`.

## Acceptance Criteria
- A user can attach a file by dragging and dropping it, clicking a button, or pasting it.
- An attached image, video, audio, or PDF file is displayed inline within the sent message.
- Attempting to attach a file larger than 50 MB results in an error message and the attachment is rejected.
- File metadata is successfully persisted in the database, and the file itself is saved to the local file system.
- Attachments flow correctly through the STOMP → NATS → broadcast pipeline and appear for all connected clients.
- The download endpoint requires authentication and serves files correctly.

## Out of Scope
- Integration with external object storage providers (e.g., S3).
- Image editing or cropping capabilities before sending.
- Multi-file selection in a single drag-and-drop or file dialog action (unless explicitly supported by the implementation without significant extra complexity).
- Orphan attachment cleanup (schema supports it, but the cleanup job is deferred).
- Message deletion and cascading file deletion.
