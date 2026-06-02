# Implementation Plan: Non-Image Attachment Rendering

> Read `AGENTS.md` (root) before starting. The coding, testing, architecture, and
> E2E rules listed there apply to every task below — they are not repeated here.

## Architectural orientation

### Root cause
`frontend/src/components/attachment_renderer.rs` is a single component that branches
on MIME type and renders `<img>`, `<video>`, `<audio>`, `<embed>` (PDF), or a generic
`<a download>` link. The `<embed>` branch is the proximate bug: many browsers ask the
user to download instead of embedding. `<video>` / `<audio>` are out of scope per
`spec.md`. The fix is to keep only an **Image** handler and a **Default** handler;
non-image attachments fall through to Default.

### Target shape
- A small pure module (`attachment_handlers`) that exposes a handler-kind enum, a
  registry-based pure resolver, and a `format_file_size` helper. No Yew, no DOM —
  testable with plain `cargo test` only.
- Two presentational components — `ImageAttachment` and `DefaultAttachment` — one
  per handler kind, each in its own file under `frontend/src/components/`.
- A thin dispatcher component (`Attachment`) that calls the resolver and renders the
  matching child. This is the **only** place that knows about the resolver.
- `AttachmentRenderer` is reduced to an iterator that delegates to `<Attachment>`.
  Adding a future handler (Video, Audio, etc.) must require touching: the enum, one
  registry row, one new component, and one match arm in `Attachment`. Nothing else.

### State / dispatch contract (re-frame style, per `AGENTS.md`)
- `ImageAttachment` reuses the existing image-click flow: dispatches
  `ChatAction::OpenImageLightbox { url, alt }` via `ChatStateContext`. No new
  actions, no new reducer arms, no new state fields are required by this track.
- `DefaultAttachment` is a pure presentational component. Download is user-initiated
  by clicking an `<a download>` (styled like a button); no JS handler, no dispatch.

### File-level plan
| Path | Status | Purpose |
|---|---|---|
| `frontend/src/attachment_handlers.rs` | new | Pure enum, registry, resolver, formatter. |
| `frontend/src/lib.rs` | edit | Register `pub mod attachment_handlers;`. |
| `frontend/src/components/image_attachment.rs` | new | Inline image handler component. |
| `frontend/src/components/default_attachment.rs` | new | Fallback metadata + download component. |
| `frontend/src/components/attachment.rs` | new | Per-attachment dispatcher (calls resolver). |
| `frontend/src/components/attachment_renderer.rs` | edit | Reduce to iterator + delegate. |
| `frontend/src/components/mod.rs` | edit | Register the three new component modules. |
| `frontend/style.scss` | edit | Add `.default-attachment` block; port renderer wrapper styles from old inline `style="..."` strings. |
| `frontend/tests/attachment_handlers_tests.rs` | new | Unit tests for resolver + formatter. |
| `e2e/tests/attachments.spec.ts` | edit | Stable-selector update + new PDF happy-path test. |

`data-testid` conventions (keep stable for E2E):
- Per-item wrapper: `attachment-{uuid}` (already exists, preserve)
- Image element: `attachment-img-{uuid}` (already exists, preserve)
- Default filename node: `attachment-filename-{uuid}` (new)
- Default download button: `attachment-download-{uuid}` (new)

---

## Phase 1: Pure handler abstraction

Goal: a self-contained, side-effect-free module that decides which handler renders
a given attachment and formats a byte count for display. Phase 1 ends green on
`cargo test -p frontend` and `cargo clippy -p frontend -- -D warnings`.

### 1.1 Write failing unit tests (Red)
- [ ] Task: Create `frontend/tests/attachment_handlers_tests.rs`. Place all tests at
  the top of the file and any fixtures (e.g. an `att_with_mime(&str) -> AttachmentMeta`
  helper) at the bottom, per project test style. Confirm the file fails to compile
  because the symbols it imports do not exist yet.

  **Resolver cases to cover:**
  - All image MIMEs resolve to `Image`: at least `image/png`, `image/jpeg`,
    `image/webp`, `image/gif`.
  - Non-image MIMEs resolve to `Default`: at least `application/pdf`, `video/mp4`,
    `audio/mpeg`, `text/plain`.
  - Unknown / generic MIME (`application/octet-stream`) resolves to `Default`.
  - Empty MIME string resolves to `Default` (degenerate input must not panic).

  **Formatter cases to cover:**
  - Zero bytes.
  - A byte count under 1 KiB (raw `B`, no decimal).
  - A byte count in the KiB / MiB / GiB ranges (one decimal each).
  - A negative input (defensive: clamp to zero, no panic). `file_size` is `i64` on
    `AttachmentMeta`, so this guard is real.

  One representative test body is enough for the executor to mirror the rest; do not
  hand them a paste-ready 16-test suite.

### 1.2 Implement the resolver (Green)
- [ ] Task: Create `frontend/src/attachment_handlers.rs` and register the module in
  `frontend/src/lib.rs`. Define:
  - `pub enum AttachmentHandlerKind { Image, Default }` (derive `Copy`, `Clone`,
    `Debug`, `PartialEq`, `Eq`).
  - A registry as a `const HANDLERS: &[(fn(&str) -> bool, AttachmentHandlerKind)]`
    with a single entry for the image matcher. The matcher is a small private
    function (e.g. `is_image_mime`) that returns `mime.starts_with("image/")`.
  - `pub fn resolve_handler(att: &AttachmentMeta) -> AttachmentHandlerKind` that
    walks `HANDLERS`, returns the first match, and falls through to `Default`.

  The "registry as ordered const slice + fall-through to Default" shape is what
  enables the "one row to add a handler" extension contract. Avoid `Box<dyn Fn>`,
  `lazy_static`, `OnceCell`, or any runtime registration — keep it static dispatch.

### 1.3 Implement `format_file_size` (Green)
- [ ] Task: `pub fn format_file_size(bytes: i64) -> String`. Use **binary** units
  (1 KB = 1024 B) to match typical OS file managers. Output shape: `"0 B"`,
  `"512 B"`, `"1.5 KB"`, `"2.5 MB"`, `"1.5 GB"`. Negative inputs clamp to zero.
  Implement as a small chain of early returns over the unit thresholds rather than
  nested `if`/`else` (per the project rule of avoiding nested control flow).

### 1.4 Green gate
- [ ] Task: All Phase 1 unit tests pass. Clippy is clean. `trunk build` still
  succeeds. The new module imports nothing from `yew::` or `web_sys::`.

### 1.5 Conductor — User Manual Verification 'Phase 1'
- [ ] Task: Conductor — User Manual Verification 'Phase 1: Pure handler
  abstraction' (Protocol in `conductor/workflow.md`).

---

## Phase 2: Handler components and dispatcher

Goal: build the two presentational components, the dispatcher, and the SCSS for the
fallback. No integration yet — `AttachmentRenderer` and `chat_window.rs` stay on the
old code path until Phase 3, so the app keeps working while these new files land.

### 2.1 `ImageAttachment` component
- [ ] Task: Create `frontend/src/components/image_attachment.rs`. Props: a single
  `AttachmentMeta`. Behaviour: render an `<img>` whose `src` is
  `/api/attachments/{id}`, whose `alt` is `original_filename`, and whose
  `data-testid` is `attachment-img-{id}`. On click, read `ChatStateContext` and
  dispatch `ChatAction::OpenImageLightbox { url, alt }` — same payload shape the
  existing renderer uses today, so the image-lightbox flow keeps working.

  The image's inline `style="max-width: 100%; max-height: 200px; display: block;"`
  in the old code should be carried over verbatim (preserves layout; SCSS migration
  is out of scope for this track).

### 2.2 `DefaultAttachment` component
- [ ] Task: Create `frontend/src/components/default_attachment.rs`. Props: a single
  `AttachmentMeta`. Visual structure (from `spec.md`):
  - A generic document icon (the same SVG path used today in the old generic
    fallback is fine — re-use, don't reinvent).
  - The filename.
  - File size via `format_file_size` and the MIME type.
  - A visible "Download" button.

  Implementation notes:
  - The download trigger is an `<a download={filename} href={url}>` styled as a
    button (CSS in 2.3). This is genuinely user-initiated — `<a download>` does
    nothing until the user clicks it, which is exactly what `spec.md` requires.
    Do **not** add `<embed>`, `<video>`, or `<audio>`.
  - Give the download element `role="button"` so screen readers announce its
    semantic role correctly.
  - Add `data-testid="attachment-filename-{id}"` to the filename element and
    `data-testid="attachment-download-{id}"` to the download element (E2E
    selectors — kept stable so the structural DOM can change later).
  - Long filenames must ellipsise rather than break the row width — handled in
    SCSS (2.3); design the markup so a flex parent + a min-width-zero meta column
    is straightforward to style.

### 2.3 SCSS additions
- [ ] Task: Append at the **end of `frontend/style.scss`**, after the existing
  `.attachment-item img { cursor: zoom-in; }` rule (matches the project convention
  used for the `.image-lightbox-*` block).

  Add two top-level rules:
  - `.attachment-renderer` + nested `.attachment-item` — port the values from the
    inline `style="..."` strings that exist today on the wrapper div and the item
    div in `attachment_renderer.rs` (flex column, 8px gap, 8px top margin; item
    with 1px solid `#ddd` border, 4px radius, 8px padding, 300px max-width). These
    move to SCSS in 3.2 once the renderer's inline styles are removed.
  - `.default-attachment` — flex row with icon column, meta column (filename + a
    sub-row of size and MIME), and a download button on the right. Use the existing
    chat-button green `#007a5a` / hover `#148567` for the download button so the UI
    feels native. Use neutral greys (`#f8f9fa` surface, `#dee2e6` border,
    `#212529` / `#6c757d` text) for the card body. Ellipsise long filenames; the
    meta column needs `min-width: 0` for that to work inside a flex item. Follow
    the Google HTML/CSS style guide (`conductor/code_styleguides/html-css.md`):
    alphabetised declarations, lowercase, hyphenated class names, no `!important`,
    no ID selectors.

### 2.4 `Attachment` dispatcher
- [ ] Task: Create `frontend/src/components/attachment.rs`. Props: a single
  `AttachmentMeta`. Body is a `match resolve_handler(&props.attachment)` with one
  arm per `AttachmentHandlerKind` variant, returning the corresponding component
  `html! { <ImageAttachment .../> }` / `html! { <DefaultAttachment .../> }`.
  This is the **only** file outside of Phase 1 that imports
  `attachment_handlers::resolve_handler`.

### 2.5 Module registration
- [ ] Task: Add the three new components to `frontend/src/components/mod.rs`. Keep
  the file's existing append-style ordering.

### 2.6 Build / lint gate
- [ ] Task: `cargo build -p frontend`, `cargo clippy -p frontend -- -D warnings`,
  `cargo test -p frontend` all green. No new unit tests are expected at the
  component level — the project relies on E2E for component behaviour (see
  `AGENTS.md`).

### 2.7 Conductor — User Manual Verification 'Phase 2'
- [ ] Task: Conductor — User Manual Verification 'Phase 2: Handler components and
  dispatcher' (Protocol in `conductor/workflow.md`).

---

## Phase 3: Integration, cleanup, and E2E

Goal: wire the new components in, delete the obsolete inline branches, prove the
bug is fixed end-to-end without regression.

### 3.1 Reduce `AttachmentRenderer` to an iterator
- [ ] Task: Replace the body of `frontend/src/components/attachment_renderer.rs`
  with a thin iterator that, for each attachment, emits the existing wrapper
  `<div class="attachment-item" data-testid="attachment-{id}">` (preserved so
  the current E2E selector keeps working) and delegates the inner rendering to
  `<Attachment attachment={att.clone()} />`. Drop:
  - All the per-MIME `if/else` branches (image, video, audio, pdf, generic).
  - The `use_context::<ChatStateContext>()` call — only `ImageAttachment` needs
    context now.
  - The inline `style="..."` attributes on the wrapper and item divs — these
    move to SCSS in 3.2.

  Result: the file should be small (well under the 15-line function rule for the
  component body) and contain no MIME knowledge.

### 3.2 Move the renderer wrapper styles into SCSS
- [ ] Task: Already specified in 2.3 — confirm the `.attachment-renderer` and
  `.attachment-item` rules are present in `frontend/style.scss` and reproduce
  the dropped inline styles exactly (no visual regression intended). The
  `.attachment-item img { cursor: zoom-in; }` rule from the image-lightbox track
  must remain — `ImageAttachment` still renders inside `.attachment-item`.

### 3.3 Build / lint / test gate
- [ ] Task: `cargo build -p frontend`, `cargo clippy -p frontend -- -D warnings`,
  `cargo test -p frontend`, and `cd frontend && trunk build` all green.

### 3.4 Stabilise the existing E2E text-file assertion
- [ ] Task: In `e2e/tests/attachments.spec.ts`, the existing "upload and receive
  attachments" test currently asserts the text-file download via
  `locator('a[download="hello.txt"]')`. That couples the test to the DOM
  attribute. Replace it with a `data-testid` lookup against
  `attachment-download-{uuid}` and a separate `toHaveAttribute('download', ...)`
  assertion. Per project rule: E2E selectors must be `data-testid` only.

### 3.5 New E2E happy-path test: PDF fallback, no auto-download
- [ ] Task: Add a second test inside the existing `test.describe('File
  Attachments', ...)` block. It should:
  - Provision two users in a new channel (mirror the two-user setup of the
    existing test).
  - Write a tiny valid-enough PDF to the temp dir (a few bytes starting with
    `%PDF-1.4` is sufficient — the renderer cares about the MIME the server
    assigns, not the bytes).
  - Attach a `page.on('download', ...)` listener on the **receiving** page **before**
    the sender sends, so any unsolicited download is captured.
  - Send the PDF; on the receiving page assert:
    - The `attachment-filename-{uuid}` element shows the filename.
    - The `attachment-download-{uuid}` element is visible and has the correct
      `download` attribute value.
    - After a short wait, the unsolicited-download list is still empty (this is
      the regression check that proves the bug is fixed).
  - Click the download element and assert a `download` event fires with the
    correct `suggestedFilename`.

  All selectors `data-testid`. No assertions on tag names or class names.

### 3.6 Run the full suite
- [ ] Task: Bring up the stack
  (`docker compose -f docker-compose.e2e.yml --project-name sana-e2e up --wait`)
  and run `cd e2e && npx playwright test --reporter=list` (the default reporter
  hangs — see `AGENTS.md`). All tests, including the existing image-lightbox
  suite, must pass.

### 3.7 Conductor — User Manual Verification 'Phase 3'
- [ ] Task: Conductor — User Manual Verification 'Phase 3: Integration, cleanup,
  and E2E' (Protocol in `conductor/workflow.md`). Walk through `spec.md`'s
  Acceptance Criteria item by item in a real browser:
  - Open a channel with a PDF attachment; confirm no automatic download dialog
    appears.
  - Confirm the fallback shows filename, formatted size, MIME, and a visible
    Download button.
  - Click the button; confirm the file downloads with the correct name.
  - Confirm images still render inline and still open the lightbox.
  - Confirm the extensibility contract by reading the code: adding a future
    handler is (a) one enum variant, (b) one row in `HANDLERS`, (c) one new
    component file, (d) one match arm in `attachment.rs` — nothing in
    `attachment_renderer.rs` or `chat_window.rs` needs to change.
