mod mutation;

pub use mutation::Mutation;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum Action {
    /// Register a mutation to an encrypted cloud.
    /// All 32 byte sequences represent a valid cloud identifier.
    /// Mutation of cloud requires proving knowledge of private key using a signature.
    /// All clouds are implicitly initialized with 0 mutations (no data).
    MutateCloud(Mutation),
    /// Authenticate as a member of a cloud. Begin receiving `CloudMutated` responses.
    /// `pubkey_hash, signature_bytes`
    AuthCloud([u8; 32], Vec<u8>),
    /// keepalive mechanism
    Ping,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum Response {
    /// `latest_known_index`
    Authenticated(u64),
    /// Notify relevant listeners that a new mutation has occurred.
    ///
    /// `latest_known_index`
    CloudMutated(u64),
    /// keepalive mechanism
    Pong,
}
