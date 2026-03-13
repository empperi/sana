---
name: messaging-infra-specialist
description: Expert in NATS JetStream, STOMP protocol, and real-time messaging patterns for Sana. Use when updating NATS consumers, implementing new real-time features, or debugging message persistence/delivery.
---

# Messaging Infra Specialist

This skill focuses on the communication backbone of Sana.

## NATS JetStream
- **Consumer Management**: Ensure consumers are configured with appropriate durability and retention policies.
- **Subject Design**: Follow the `topic.<channel_id>` naming convention for messages.

## STOMP Integration
- **Protocol Fidelity**: Adhere to the STOMP 1.2 specification for frame construction.
- **Heartbeats**: Implement and monitor heartbeats to maintain stable WebSocket connections.

## Resilience
- **Reconnection Logic**: Ensure the client and archiver can gracefully recover from NATS or server restarts.
- **Message Ordering**: Leverage JetStream sequence numbers for strict ordering where required.

## Testing
- **Mock NATS**: Use `nats-server` in tests or a mock client for faster verification of messaging logic.
