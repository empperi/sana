# Specification: Non-Image Attachment Rendering

### Overview
Currently, non-image attachments (e.g., PDFs) are rendered inline as if they were images. This causes the browser to automatically trigger unintended file downloads (a "save as" dialog) immediately upon rendering the chat message. This track will fix this by introducing a component-based file handler abstraction in the frontend. This architecture will allow different file types to be handled by specific UI components. In this iteration, we will implement two handlers: an Image handler (retaining current inline visual behavior) and a Default fallback handler (displaying file metadata and a manual download button) for all other file types.

### Functional Requirements
1. **Attachment Handler Abstraction:** Introduce an extensible frontend architecture to register and resolve different attachment handlers based on file type or MIME type.
2. **Image Handler Component:** Extract the existing image rendering logic into a dedicated "Image Handler" component. This handler will be responsible for displaying inline images (e.g., png, jpg, webp, gif).
3. **Default Fallback Component:** Create a generic file handler component for all non-image attachments.
   - It must display the file name, file size (formatted nicely), and file type.
   - It must include a user-initiated "Download" button to explicitly trigger the file download, preventing automatic browser downloads upon rendering.
4. **Message Rendering Integration:** Update the message rendering logic to pass attachment data to the handler abstraction, which will dynamically select and render the appropriate handler component.

### Non-Functional Requirements
1. **Extensibility:** The handler abstraction must be designed so that adding future handlers (e.g., for Video or Audio) requires minimal changes to the core message rendering logic.
2. **Maintainability:** The new frontend architecture should align with Yew's component model and the project's state management guidelines.

### Acceptance Criteria
- [ ] Users can view a chat containing a non-image file (e.g., a PDF) without the browser automatically prompting a file download.
- [ ] Non-image files are displayed using the generic fallback UI, showing the file name, size, type, and a functioning download button.
- [ ] Clicking the download button on a non-image attachment successfully downloads the file.
- [ ] Images continue to render inline properly using the newly abstracted Image Handler.
- [ ] The codebase clearly demonstrates an extensible architecture for adding new file type handlers in the future.

### Out of Scope
- Implementing specific handlers for Video or Audio files.
- Backend changes to how attachments are stored or served.
- Changes to the attachment upload process.