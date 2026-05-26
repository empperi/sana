# Image Lightbox Specification

## Overview

The file attachment feature already renders images inline within the chat at a constrained size (`max-height: 200px`).
This track extends that capability: when a user clicks (or taps on mobile) an inline image, the image opens in a
full-screen lightbox modal showing the image at its maximum possible size, with stylish margins.

## UX Requirements

### Opening the lightbox
- Every inline image rendered by `AttachmentRenderer` must be clickable.
- Clicking/tapping an image opens the lightbox with that image.
- The image `alt` text must be preserved for accessibility.

### Lightbox appearance
- A **semi-transparent black overlay** covers the entire viewport behind the image (`rgba(0,0,0,0.85)` or similar).
- The image is **centred** in the viewport and scaled to fill most of the available screen estate with comfortable
  margins (e.g., `max-width: 90vw`, `max-height: 90vh`, preserving aspect ratio).
- A **close button (`×`)** is displayed in the **upper-left corner** of the image as a visual affordance. It must
  be styled as a semi-transparent button (white icon, dark circular or pill background) so it is legible on all
  image colours. The button must have `data-testid="lightbox-close-button"`.

### Closing the lightbox
All three of the following mechanisms must close the lightbox:
1. **Clicking the overlay** (anywhere outside the image itself).
2. **Pressing the `Escape` key** on the keyboard.
3. **Browser back navigation** — pressing the hardware/software back button on mobile, or using the browser's Back
   gesture. Implementation note: when the lightbox opens, push a history entry (`history.pushState`). On `popstate`
   event the lightbox closes. If the user closes via overlay/Escape, call `history.back()` to pop that entry so the
   navigation stack stays clean.

### Accessibility
- The overlay element must have `role="dialog"`, `aria-modal="true"`, and `aria-label="Image preview"`.
- Focus must be trapped within the lightbox while open (at minimum the close button should receive focus on open).
- Keyboard `Escape` must close the lightbox without any additional UI interaction.

### Mobile
- Touch tapping an inline image must open the lightbox (standard browser touch-click parity handles this; no
  special touch handling is needed beyond the normal `onclick`).
- The browser back gesture must close the lightbox (covered by the `popstate` handler).

## Architecture Constraints

This is a **frontend-only** change. No backend modifications are required.

All implementation must follow the existing Yew architecture:
- **Global state** (`ChatState` via `use_reducer`): The lightbox open/closed state and the URL of the image being
  previewed must live in `ChatState`, not in component-local state.
- **Actions** (`ChatAction` enum): New variants `OpenImageLightbox { url: String, alt: String }` and
  `CloseImageLightbox` control the state.
- **Business logic** (`src/logic.rs`): Any non-trivial imperative work (e.g., history push/pop) must be in the
  logic or hooks layer, not directly inside the component.
- **Component** (`image_lightbox.rs`): Pure rendering + event wiring. Registered in `components/mod.rs` and
  rendered at the application root level (alongside `JoinChannelModal`) so it is not nested inside the scroll
  container.
- **Styling** (`style.scss`): New SCSS block `.image-lightbox-*`. Must not conflict with the existing
  `.modal-overlay`/`.modal-content` blocks used by `JoinChannelModal`.

## Test Requirements

### Unit tests
- State reducer: `OpenImageLightbox` sets `lightbox_image.url` and `lightbox_image.alt`; `CloseImageLightbox`
  clears them.
- Follow the project convention: test files are separate (not `mod tests` inline), tests go first, fixtures after.

### E2E tests (Playwright)
- Happy path: upload an image via the attachment button → send message → click the inline image →
  lightbox overlay appears → image is visible inside.
- Close via overlay click.
- Close via Escape key.
- Close via the `×` button.
- Use `data-testid` selectors throughout. Never use CSS classes or DOM structure.
- Add to `e2e/tests/image_lightbox.spec.ts`.

## Out of Scope
- Zoom / pan inside the lightbox.
- Gallery navigation (next/previous image).
- Download button inside the lightbox (the existing `<a download>` link in `AttachmentRenderer` covers downloads).
- Video or audio lightbox (only images).
