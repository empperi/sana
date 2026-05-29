# Implementation Plan: Non-Image Attachment Rendering

### Phase 1: Attachment Handler Architecture
- [ ] Task: Design and Implement Handler Abstraction
    - [ ] Create failing unit tests for the attachment handler registry and resolution logic based on file/MIME type (Red Phase).
    - [ ] Implement the abstract handler trait/type and the core registry in the frontend (Green Phase).
- [ ] Task: Conductor - User Manual Verification 'Attachment Handler Architecture' (Protocol in workflow.md)

### Phase 2: Core Components Implementation
- [ ] Task: Extract Image Handler Component
    - [ ] Write failing unit tests for a standalone `ImageAttachment` component (Red Phase).
    - [ ] Migrate existing inline image rendering logic into the `ImageAttachment` component (Green Phase).
- [ ] Task: Implement Default Fallback Component
    - [ ] Write failing unit tests verifying the `DefaultAttachment` component renders file metadata and a download button (Red Phase).
    - [ ] Implement the `DefaultAttachment` component with explicit download invocation (Green Phase).
- [ ] Task: Conductor - User Manual Verification 'Core Components Implementation' (Protocol in workflow.md)

### Phase 3: Integration and E2E Testing
- [ ] Task: Update Message Rendering Integration
    - [ ] Write failing tests ensuring the main `Message` component dynamically selects the correct handler via the registry (Red Phase).
    - [ ] Update `Message` rendering logic to utilize the handler abstraction instead of hardcoding image rendering (Green Phase).
- [ ] Task: E2E Verification
    - [ ] Add/Update Playwright E2E tests to upload and view a non-image attachment, verifying no automatic download occurs and the fallback UI is present.
    - [ ] Execute the full Playwright E2E suite to confirm no regressions in chat rendering.
- [ ] Task: Conductor - User Manual Verification 'Integration and E2E Testing' (Protocol in workflow.md)