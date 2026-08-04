use patchbay_contracts::patchbay::{
    ResourceStateEvent, StoredEventKind, StoredEventPayload,
};
use prost::Message;

/// Encode one normalized resource-state update in the schema-owned durable
/// event envelope.
#[must_use]
pub fn encode(event: &ResourceStateEvent) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::ResourceState as i32,
        payload: event.encode_to_vec(),
    }
}
