# Implementation Plan — Image Lightbox

## Key Architecture Reference

Before starting any phase, understand these patterns:

- **Global state:** `ChatState` in `frontend/src/logic.rs` holds all app state. New lightbox state goes here.
- **Actions:** `ChatAction` enum (same file) drives all state changes. Add `OpenImageLightbox` / `CloseImageLightbox`.
- **Reducer:** `frontend/src/state.rs` — `Reducible` impl delegates to `ChatState` methods. Add reducer arms here.
- **Components:** Pure rendering; never own business state. Register in `frontend/src/components/mod.rs`.
- **Mounting:** The `Chat` route renders `ChatApp` (in `frontend/src/main.rs`), which calls `render_app` and returns
  the `<div class="app-container">` containing `Sidebar`, `ChatWindow`, and `JoinChannelModal`. The `ImageLightbox`
  component must be added inside this same `app-container` (alongside `JoinChannelModal`). It will still cover the
  full viewport because its overlay uses `position: fixed`.
- **SCSS:** All styles live in `frontend/style.scss`. Add a new `.image-lightbox-*` block; never reuse or
  modify `.modal-overlay` / `.modal-content` (those belong to `JoinChannelModal`).
- **Tests:** Backend — `tests/`. Frontend unit tests — separate test files in `frontend/tests/`, not inline
  `mod tests`. E2E — `e2e/tests/`.
- **Existing image URL:** Inline images already render with `src="/api/attachments/{id}"` and load successfully
  with cookie auth (browsers send same-origin cookies on `<img>` requests). The lightbox uses the same URL — no
  backend changes are required.

---

## Phase 1: Global State and Actions [checkpoint: ]

### 1.1 Add lightbox state fields to `ChatState`
- [ ] Task: In `frontend/src/logic.rs`, immediately above the `ChatState` struct definition (around line 99), add a
  new struct:
  ```rust
  #[derive(Clone, PartialEq, Debug, Default)]
  pub struct LightboxImage {
      pub url: String,
      pub alt: String,
  }
  ```
- [ ] Task: In `ChatState`, add a new field:
  ```rust
  pub lightbox_image: Option<LightboxImage>,
  ```
- [ ] Task: In `ChatState::new()`, initialise the new field to `None`:
  ```rust
  lightbox_image: None,
  ```

### 1.2 Add new `ChatAction` variants
- [ ] Task: In `frontend/src/logic.rs`, add to the `ChatAction` enum (the `Default`-derived `Clone, PartialEq, Debug`
  enum at line 117):
  ```rust
  OpenImageLightbox { url: String, alt: String },
  CloseImageLightbox,
  ```

### 1.3 Implement state mutation methods
- [ ] Task: In `frontend/src/logic.rs`, in `impl ChatState`, add two methods:
  ```rust
  pub fn open_lightbox(&mut self, url: String, alt: String) {
      self.lightbox_image = Some(LightboxImage { url, alt });
  }

  pub fn close_lightbox(&mut self) {
      self.lightbox_image = None;
  }
  ```
  These are pure state mutations only. Side effects (history, focus, scroll lock) live in the component.
  `close_lightbox` is idempotent — calling it on a state where `lightbox_image` is already `None` must remain a
  no-op.

### 1.4 Add reducer arms
- [ ] Task: In `frontend/src/state.rs`, inside the `match action` block of `Reducible::reduce`, add arms (after
  the existing `SetAttachmentError` arm):
  ```rust
  ChatAction::OpenImageLightbox { url, alt } => {
      new_state.open_lightbox(url, alt);
  }
  ChatAction::CloseImageLightbox => {
      new_state.close_lightbox();
  }
  ```

### 1.5 Write unit tests for lightbox state
- [ ] Task: Add tests to the **existing** `frontend/tests/state_tests.rs` (do not create a new test file —
  state-related action tests already live there; one fewer file to register). Append four tests:
  1. `test_open_image_lightbox_sets_state`: dispatch `OpenImageLightbox { url: "/api/attachments/abc".into(), alt:
     "photo.png".into() }` on a fresh `ChatState`. Assert `new_state.lightbox_image == Some(LightboxImage {
     url: "/api/attachments/abc".into(), alt: "photo.png".into() })`.
  2. `test_close_image_lightbox_clears_state`: build a state with `lightbox_image = Some(...)`, dispatch
     `CloseImageLightbox`, assert `new_state.lightbox_image.is_none()`.
  3. `test_open_image_lightbox_replaces_existing`: dispatch `OpenImageLightbox` twice with different urls; assert
     the second url is the one stored.
  4. `test_close_image_lightbox_is_noop_when_already_closed`: dispatch `CloseImageLightbox` on a fresh state.
     Assert `new_state.lightbox_image.is_none()` and the state otherwise equals `ChatState::new()` (use
     `assert_eq!(*new_state, ChatState::new())`).
- [ ] Task: Write tests **first** (Red phase). Confirm they fail. Implement 1.1–1.4. Confirm they pass (Green phase).

### 1.6 Conductor — User Manual Verification 'Phase 1'
- [ ] Task: Conductor — User Manual Verification 'Phase 1: Global State and Actions' (Protocol in workflow.md).
  Verify:
  - `cargo test -p frontend` passes including the four new tests.
  - `cargo clippy -p frontend -- -D warnings` produces zero warnings.
  - `cd frontend && trunk build` compiles cleanly.

---

## Phase 2: `ImageLightbox` Component and Styling [checkpoint: ]

### 2.1 Create SCSS styles for the lightbox
- [ ] Task: In `frontend/style.scss`, append the following block at the **end of the file** (after all existing
  rules — placement at the end avoids ordering surprises with media queries):
  ```scss
  // Image Lightbox
  .image-lightbox-overlay {
      position: fixed;
      inset: 0;
      background-color: rgba(0, 0, 0, 0.85);
      display: flex;
      justify-content: center;
      align-items: center;
      z-index: 1100; // Above .modal-overlay (1000)
      cursor: zoom-out;

      .image-lightbox-container {
          position: relative;
          display: inline-flex; // Shrinks to image size
          cursor: default;      // Reset cursor inside container
          max-width: 90vw;
          max-height: 90vh;
      }

      .image-lightbox-img {
          display: block;
          max-width: 90vw;
          max-height: 90vh;
          object-fit: contain;
          border-radius: 4px;
          box-shadow: 0 8px 40px rgba(0, 0, 0, 0.6);
      }

      .image-lightbox-close {
          position: absolute;
          top: 8px;
          left: 8px;
          width: 32px;
          height: 32px;
          border-radius: 50%;
          background-color: rgba(0, 0, 0, 0.7);
          border: 2px solid rgba(255, 255, 255, 0.8);
          color: white;
          font-size: 1.1em;
          line-height: 1;
          cursor: pointer; // Explicit — overrides parent's zoom-out
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 1101;
          transition: background-color 0.15s ease;

          &:hover {
              background-color: rgba(0, 0, 0, 0.9);
          }

          &:focus-visible {
              outline: 2px solid white;
              outline-offset: 2px;
          }
      }
  }

  // Make inline attachment images show a pointer to invite clicking
  .attachment-item img {
      cursor: zoom-in;
  }
  ```
  **Important:** the close button is positioned **inside** the image (top:8px, left:8px), not outside it. This
  prevents clipping off-screen on small viewports and matches the spec wording ("upper-left corner of the image").

### 2.2 Create the `ImageLightbox` component — file scaffold
- [ ] Task: Create `frontend/src/components/image_lightbox.rs` with the imports below. Read carefully — every
  detail matters:
  ```rust
  use yew::prelude::*;
  use std::rc::Rc;
  use std::cell::RefCell;
  use wasm_bindgen::prelude::*;
  use wasm_bindgen::JsCast;
  use web_sys::{HtmlElement, KeyboardEvent, MouseEvent};
  use crate::logic::ChatAction;
  use crate::state::ChatStateContext;
  ```

### 2.3 Component body — read context and short-circuit when closed
- [ ] Task: Inside `image_lightbox.rs`, define the function component. The component takes **no props**:
  ```rust
  #[function_component(ImageLightbox)]
  pub fn image_lightbox() -> Html {
      let ctx = use_context::<ChatStateContext>().expect("ChatStateContext not found");
      let lightbox = ctx.state.lightbox_image.clone();

      // Refs that survive across renders. We use Rc<RefCell<...>> because we need to mutate
      // these values inside event listener closures, and use_state values would be captured
      // as stale snapshots inside those closures.
      let history_pushed = use_mut_ref(|| false);
      let previous_focus = use_mut_ref(|| None::<web_sys::Element>);
      let close_button_ref = use_node_ref();

      // ... see 2.4 for the effect, 2.5 for the close handler, 2.6 for the html! return
  }
  ```

### 2.4 The `use_effect_with` that installs/uninstalls side effects
- [ ] Task: Still in the component, add the effect. The dependency is `lightbox.is_some()` (a `bool`) so the
  effect re-runs only when the lightbox transitions open ↔ closed — not when the URL/alt changes within an open
  state. (We do not currently support replacing the open image without closing first; if you ever do, the effect
  must be revisited.)
  ```rust
  {
      let dispatch = ctx.dispatch.clone();
      let history_pushed = history_pushed.clone();
      let previous_focus = previous_focus.clone();
      let close_button_ref = close_button_ref.clone();
      let is_open = lightbox.is_some();

      use_effect_with(is_open, move |&is_open| {
          // Holders that the cleanup closure can move out of.
          let mut keydown_listener: Option<gloo_events::EventListener> = None;
          let mut popstate_listener: Option<gloo_events::EventListener> = None;
          let mut prev_body_overflow: Option<String> = None;

          if is_open {
              let window = web_sys::window().expect("no window");
              let document = window.document().expect("no document");
              let body = document.body().expect("no body");

              // 1. Capture the currently focused element so we can restore it on close.
              *previous_focus.borrow_mut() = document.active_element();

              // 2. Lock body scroll.
              prev_body_overflow = body.style().get_property_value("overflow").ok();
              let _ = body.style().set_property("overflow", "hidden");

              // 3. Push a history entry so the back button closes the lightbox.
              //    Pass empty string for url to keep the current URL (no #lightbox hash —
              //    that would scroll to an anchor and persist in the bar after reload).
              let history = window.history().expect("no history");
              let _ = history.push_state_with_url(&JsValue::NULL, "", Some(""));
              *history_pushed.borrow_mut() = true;

              // 4. Focus the close button after the DOM paints.
              if let Some(btn) = close_button_ref.cast::<HtmlElement>() {
                  let _ = btn.focus();
              }

              // 5. Install Escape key listener.
              {
                  let dispatch = dispatch.clone();
                  let history_pushed = history_pushed.clone();
                  keydown_listener = Some(gloo_events::EventListener::new(
                      &window,
                      "keydown",
                      move |event| {
                          let event = event.dyn_ref::<KeyboardEvent>().unwrap();
                          if event.key() == "Escape" {
                              event.prevent_default();
                              close_with_history_pop(&dispatch, &history_pushed);
                          }
                      },
                  ));
              }

              // 6. Install popstate listener. The browser already navigated back when this
              //    fires, so we MUST NOT call history.back() again — just dispatch close.
              {
                  let dispatch = dispatch.clone();
                  let history_pushed = history_pushed.clone();
                  popstate_listener = Some(gloo_events::EventListener::new(
                      &window,
                      "popstate",
                      move |_event| {
                          *history_pushed.borrow_mut() = false;
                          dispatch.emit(ChatAction::CloseImageLightbox);
                      },
                  ));
              }
          }

          // Cleanup runs when is_open changes (true→false) or on component drop.
          move || {
              // Drop listeners (gloo_events::EventListener detaches on drop).
              drop(keydown_listener);
              drop(popstate_listener);

              if is_open {
                  let window = web_sys::window().expect("no window");
                  let document = window.document().expect("no document");
                  let body = document.body().expect("no body");

                  // Restore body scroll.
                  match &prev_body_overflow {
                      Some(v) if !v.is_empty() => {
                          let _ = body.style().set_property("overflow", v);
                      }
                      _ => {
                          let _ = body.style().remove_property("overflow");
                      }
                  }

                  // Restore focus to the element that opened the lightbox.
                  if let Some(el) = previous_focus.borrow_mut().take() {
                      if let Ok(html_el) = el.dyn_into::<HtmlElement>() {
                          let _ = html_el.focus();
                      }
                  }

                  // Cleanup runs because state went open→closed. The state change came
                  // from one of: overlay click, X button, Escape, or popstate.
                  // - For the first three, close_with_history_pop already called
                  //   history.back() and cleared the flag.
                  // - For popstate, the popstate handler cleared the flag.
                  // So if `history_pushed` is somehow still true here (e.g., the
                  // component is unmounting because the user navigated away), we
                  // should pop the entry to keep the stack clean.
                  if *history_pushed.borrow() {
                      let history = window.history().expect("no history");
                      let _ = history.back();
                      *history_pushed.borrow_mut() = false;
                  }
              }
          }
      });
  }
  ```

### 2.5 The shared close helper
- [ ] Task: At module level (top of `image_lightbox.rs`, above the component fn), define:
  ```rust
  fn close_with_history_pop(
      dispatch: &Callback<ChatAction>,
      history_pushed: &Rc<RefCell<bool>>,
  ) {
      // Order matters: clear flag BEFORE history.back() so the popstate handler
      // (which may fire synchronously in some browsers) sees the cleared flag.
      let was_pushed = *history_pushed.borrow();
      *history_pushed.borrow_mut() = false;

      if was_pushed {
          if let Some(history) = web_sys::window().and_then(|w| w.history().ok()) {
              let _ = history.back();
          }
      }

      dispatch.emit(ChatAction::CloseImageLightbox);
  }
  ```
  Note: the popstate listener will also dispatch `CloseImageLightbox` in response to `history.back()`. Dispatching
  twice is safe because `close_lightbox` is idempotent (verified by test 4 in Phase 1.5).

### 2.6 Render the html
- [ ] Task: Inside the component fn (after the `use_effect_with` block), build the close handler and the html
  output:
  ```rust
  let on_close = {
      let dispatch = ctx.dispatch.clone();
      let history_pushed = history_pushed.clone();
      Callback::from(move |_e: MouseEvent| {
          close_with_history_pop(&dispatch, &history_pushed);
      })
  };

  let stop = Callback::from(|e: MouseEvent| e.stop_propagation());

  match lightbox {
      None => html! {},
      Some(img) => html! {
          <div class="image-lightbox-overlay"
               role="dialog"
               aria-modal="true"
               aria-label="Image preview"
               data-testid="image-lightbox-overlay"
               onclick={on_close.clone()}>
              <div class="image-lightbox-container" onclick={stop}>
                  <button class="image-lightbox-close"
                          ref={close_button_ref}
                          data-testid="lightbox-close-button"
                          aria-label="Close image preview"
                          onclick={on_close.clone()}>
                      { "×" }
                  </button>
                  <img class="image-lightbox-img"
                       data-testid="lightbox-image"
                       src={img.url}
                       alt={img.alt} />
              </div>
          </div>
      },
  }
  ```

### 2.7 Verify dependencies
- [ ] Task: Confirm `gloo-events` is already in `frontend/Cargo.toml`. If not, add it under `[dependencies]`:
  ```toml
  gloo-events = "0.2"
  ```
  Run `cd frontend && cargo build` to verify the crate resolves.

### 2.8 Register `ImageLightbox` in `mod.rs` and mount inside `app-container`
- [ ] Task: In `frontend/src/components/mod.rs`, add `pub mod image_lightbox;` to the existing list.
- [ ] Task: In `frontend/src/main.rs`:
  - Add `use frontend::components::image_lightbox::ImageLightbox;` near the other component imports (~line 13).
  - In `render_app` (around line 281), inside the existing `<div class="app-container">`, add `<ImageLightbox />`
    after the `<JoinChannelModal>` element but **before** the closing `</div>` of `app-container`. The component
    has no props.

### 2.9 Conductor — User Manual Verification 'Phase 2'
- [ ] Task: Conductor — User Manual Verification 'Phase 2: ImageLightbox Component and Styling' (Protocol in
  workflow.md). Verify:
  - `cd frontend && trunk build` succeeds.
  - `cargo clippy -p frontend -- -D warnings` produces zero warnings.
  - The component renders nothing yet because no action dispatches `OpenImageLightbox` (Phase 3 wires that up).

---

## Phase 3: Wire `AttachmentRenderer` to dispatch lightbox [checkpoint: ]

### 3.1 Read `ChatStateContext` inside `AttachmentRenderer` (no prop change)
- [ ] Task: Modify `frontend/src/components/attachment_renderer.rs` as follows. **Do not change the props
  signature** — `AttachmentRenderer` continues to accept only `attachments`. Instead, read the context inside the
  function (this matches the existing pattern in `attachment_button.rs:10`).
- [ ] Task: Add imports at the top:
  ```rust
  use crate::state::ChatStateContext;
  use crate::logic::ChatAction;
  use web_sys::MouseEvent;
  ```
- [ ] Task: At the start of `attachment_renderer`, add:
  ```rust
  let ctx = use_context::<ChatStateContext>().expect("No ChatStateContext found");
  ```
- [ ] Task: Inside the `if mime.starts_with("image/")` branch, replace the bare `<img>` with one that has an
  `onclick` handler and a unique `data-testid`. The clone setup mirrors the existing `id_str`/`url` pattern:
  ```rust
  let url_clone = url.clone();
  let alt_clone = att.original_filename.clone();
  let dispatch = ctx.dispatch.clone();
  let img_testid = format!("attachment-img-{}", id_str);
  let on_img_click = Callback::from(move |_e: MouseEvent| {
      dispatch.emit(ChatAction::OpenImageLightbox {
          url: url_clone.clone(),
          alt: alt_clone.clone(),
      });
  });
  ```
  Then in the `html!`:
  ```rust
  <img src={url.clone()}
       alt={att.original_filename.clone()}
       data-testid={img_testid}
       onclick={on_img_click}
       style="max-width: 100%; max-height: 200px; display: block;" />
  ```
- [ ] Task: Leave the `video`, `audio`, `embed`, and `<a download>` branches untouched.

### 3.2 Verify context provider tree
- [ ] Task: Confirm the rendering chain reaches `AttachmentRenderer` inside the `ChatStateProvider`:
  `App` → `BrowserRouter` → `ChatStateProvider` → `Switch` → `ChatApp` → `render_app` → `ChatWindow` →
  `AttachmentRenderer`. `chat_window.rs:29` already calls `use_context::<ChatStateContext>()`, proving the
  provider is in scope. No changes needed.

### 3.3 Update existing tests if any consume `AttachmentRenderer`
- [ ] Task: Run `grep -rn "AttachmentRenderer" frontend/tests/ frontend/src/`. If any test instantiates
  `AttachmentRenderer` directly (without a `ChatStateProvider`), the test will panic at the new `use_context`
  call. Wrap such tests in a `<ChatStateProvider>` host. If no tests reference it directly, nothing to do.

### 3.4 Conductor — User Manual Verification 'Phase 3'
- [ ] Task: Conductor — User Manual Verification 'Phase 3: Wire AttachmentRenderer' (Protocol in workflow.md).
  Verify in the browser:
  - Send a message with an image attachment.
  - Click the inline image — lightbox opens with the full image.
  - Click outside the image — lightbox closes.
  - Open again, press Escape — lightbox closes.
  - Open again, click the × button — lightbox closes.
  - Open again, press the browser back button — lightbox closes and the URL has not changed (no `#lightbox` in
    the address bar).
  - After closing, focus is restored to the thumbnail (verify by pressing Tab — it should land on the next
    interactive element after the image, not at `<body>`).
  - Body scroll is locked while the lightbox is open (try scrolling — chat behind should not move).
  - `cargo clippy -p frontend -- -D warnings` produces zero warnings.
  - No console errors in devtools.

---

## Phase 4: E2E Tests and Final Polish [checkpoint: ]

### 4.1 Write E2E happy-path tests
- [ ] Task: Create `e2e/tests/image_lightbox.spec.ts`. Reuse the upload pattern from
  `e2e/tests/attachments.spec.ts`. Selectors must use `data-testid` exclusively. Tests:
  1. **Open and close via X button** (primary happy path):
     - Login as user A. Upload a small JPEG (reuse the helper / pattern from `attachments.spec.ts` lines that
       upload `test.png`). Send the message.
     - Wait for the inline image to appear: `page.getByTestId(/^attachment-img-/).first()`.
     - Click it.
     - Assert `page.getByTestId('image-lightbox-overlay')` is visible.
     - Assert `page.getByTestId('lightbox-image')` is visible.
     - Click `page.getByTestId('lightbox-close-button')`.
     - Assert `page.getByTestId('image-lightbox-overlay')` is no longer visible.
  2. **Close via Escape**: open as above, then `await page.keyboard.press('Escape')`. Assert overlay gone.
  3. **Close via overlay click**: open as above, then click the overlay at a corner so the click does not hit
     the inner container:
     ```ts
     await page.getByTestId('image-lightbox-overlay').click({ position: { x: 5, y: 5 } });
     ```
     Assert overlay gone. Note: the inner container has `e.stopPropagation()` so clicking the image itself
     would NOT close — only clicks on the overlay's own area do.
  4. **Close via browser back**: open as above, then `await page.goBack()`. Assert overlay gone. Assert the URL
     equals the URL before opening (no leftover `#lightbox`).

### 4.2 Verify mobile viewport behaviour
- [ ] Task: Add a fifth test in the same spec file that calls
  `await page.setViewportSize({ width: 390, height: 844 })` before uploading, then runs the open + close-via-X
  flow. Assert the close button is visible and clickable (`await expect(page.getByTestId('lightbox-close-button'))
  .toBeVisible()`).

### 4.3 Run E2E tests
- [ ] Task: Ensure the Docker stack is running:
  ```
  docker compose -f docker-compose.e2e.yml --project-name sana-e2e up --wait
  ```
  Then:
  ```
  cd e2e && npx playwright test image_lightbox --reporter=list
  ```
  All five tests must pass. **Always** use `--reporter=list` (the default HTML reporter hangs on a server).

### 4.4 Final build and lint gate
- [ ] Task: Run the full quality gate from the project root:
  - `cargo test` (backend — confirms nothing else broke)
  - `cargo clippy -- -D warnings` (zero warnings required)
  - `cd frontend && cargo test` (frontend unit tests, including the four lightbox tests from Phase 1.5)
  - `cd frontend && cargo clippy -- -D warnings`
  - `cd frontend && trunk build` (Wasm compile)
  - `cd e2e && npx playwright test --reporter=list` (full E2E suite — confirms no regressions in other suites)
  Fix any issues found. Do not mark the track complete until all checks pass.

### 4.5 Conductor — User Manual Verification 'Phase 4'
- [ ] Task: Conductor — User Manual Verification 'Phase 4: E2E Tests and Final Polish' (Protocol in workflow.md).
  Full walkthrough:
  - Two browser windows open to the app. User A sends a message with an image. User B sees it inline and can
    open the lightbox.
  - All four close mechanisms work (overlay, Escape, X, back gesture).
  - Body scroll is locked while open; restored on close.
  - Focus restored to the thumbnail after close.
  - No console errors.
  - Mobile viewport (devtools responsive mode, e.g. iPhone 12) — image not clipped, close button reachable.
  - All E2E tests green.
