# Security: Channel Authorization & Server-Side Sessions — Specification

## Overview

The 2026-07-13 architecture review (`docs/architecture-review-2026-07-13.md`, findings B1/B2/B13) identified two
security gaps:

1. **No channel authorization on the WebSocket path.** Any authenticated user can `SUBSCRIBE` to any channel
   (receiving its full history) and `SEND` to any channel without having joined it. Membership is checked only in
   the REST history endpoint. The `is_private` flag exists in the schema but is enforced nowhere. Attachment
   downloads have no ownership or membership check.
2. **Sessions are not real sessions.** The cookie is a signed `user_id` with no expiry and no server-side record.
   Logout only clears a local cache; a captured cookie remains valid forever. The cookie also lacks `HttpOnly`,
   `SameSite`, and `Secure` attributes.

This track introduces DB-backed sessions and a single, reusable channel-authorization service function that all
transports (REST and WebSocket) call.

## Requirements

### Server-side sessions

- New `sessions` table: `id UUID PK DEFAULT gen_random_uuid()`, `user_id UUID NOT NULL REFERENCES users(id)
  ON DELETE CASCADE`, `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`, `expires_at TIMESTAMPTZ NOT NULL`.
  Index on `user_id` (to delete all sessions of a user later).
- Session lifetime: **30 days**, fixed (no sliding renewal in this track). Expressed as a named constant.
- Login and register create a session row and set the cookie to the **session id** (not the user id). The cookie
  remains a `SignedCookieJar` cookie named `session_id`.
- Logout deletes the session row (and clears the local cache) — a replayed cookie must be rejected after logout.
- Validation resolves the session id to a `user_id`, rejecting missing or expired sessions. Expired rows may be
  lazily deleted on lookup; a background sweeper is out of scope.
- The existing 60-second in-memory validation cache is kept, but re-keyed by session id and storing
  `(user_id, cached_at)`. **Accepted tradeoff (documented):** after logout, *other* replicas may accept the
  session from their cache for up to 60 seconds. This window is acceptable for now; anything requiring shared
  cache infrastructure (Redis etc.) is out of scope.
- Cookie attributes: `HttpOnly`, `SameSite=Lax`, `Path=/`, and `Secure` controlled by a new config value
  `cookie_secure` (env `COOKIE_SECURE`, default `false` because local dev runs over plain HTTP).

### Channel authorization

- One service-layer function is the single source of truth for "may this user access this channel", used by every
  transport. It checks the `user_channels` membership table.
- WebSocket `SUBSCRIBE`: non-members receive a STOMP `ERROR` frame and no subscription is created.
  `system.channels` remains available to every authenticated user.
- WebSocket `SEND` (both chat messages and read markers): non-members receive an `ERROR` frame and nothing is
  published to NATS.
- Per-connection membership cache: once a channel is verified for a socket, subsequent sends on that socket skip
  the DB check (membership is currently never revoked — there is no "leave channel" feature).
- REST `GET /api/channels/:id/messages` switches its inline membership check to the shared function (behavior
  unchanged).
- Attachment download (`GET /api/attachments/:id`): if the attachment is linked to a message, the requester must
  be a member of that message's channel; if it is unlinked (pending), only the uploader may download it.
- Referencing attachments in a message (`attachment_ids` in the send payload): only attachments uploaded by the
  sender are accepted; foreign IDs are silently dropped from the message.
- Private channels (partial enforcement until invitations exist): excluded from the unjoined-channel search, and
  `POST /api/channels/join` returns `403` for a private channel.

## Acceptance criteria

1. Logging out, then replaying the old cookie against `/api/auth/me`, `/api/channels`, and `/ws` yields `401`.
2. A session older than 30 days is rejected (test by inserting an expired row directly).
3. Set-Cookie on login/register carries `HttpOnly`, `SameSite=Lax`, and `Path=/` (plus `Secure` when
   `COOKIE_SECURE=true`).
4. A user who has not joined channel X: WS `SUBSCRIBE` to X returns an `ERROR` frame and no history; WS `SEND` to
   X returns an `ERROR` frame and the message is not persisted or broadcast.
5. A member of X can subscribe and send exactly as before (no regression in the existing E2E suite, which runs
   against 2 app replicas — cross-instance behavior must keep working).
6. Downloading an attachment linked to a message in a channel the requester has not joined returns `403`.
7. All existing backend tests pass after being updated to create sessions instead of raw user-id cookies.

## Out of scope

- Session rotation, refresh tokens, "remember me" variants, bearer-token auth (planned separately for mobile).
- Shared cross-replica session cache (Redis).
- Channel invitations / full private-channel feature.
- Rate limiting.
- Revoking live WebSocket connections when a session is deleted mid-connection.
