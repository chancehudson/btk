use serde::Deserialize;
use serde::Serialize;

/// Public data for a mutation to an encrypted cloud.
/// Used to ensure consistency among synchronized devices.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MutationMetadata {
    /// Global counter of mutations
    index: u64,
    /// Encrypted mutation/diff/action
    data: Vec<u8>,
    /// Variable length signature, impl defined algo
    signature: Vec<u8>,
    /// 32 byte public key hash, impl defined algo
    public_key_hash: [u8; 32],
    /// Optional full public key. This must be provided if `index == 0` as the encrypted cloud is
    /// being created.
    public_key: Option<Vec<u8>>,
}

impl MutationMetadata {
    pub fn hash() -> [u8; 32] {
        unimplemented!();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum Action {
    /// Register a mutation to an encrypted cloud.
    /// All 32 byte sequences represent a valid cloud identifier.
    /// Mutation of cloud requires proving knowledge of private key using a signature.
    /// All clouds are implicitly initialized with 0 mutations (no data).
    MutateCloud(MutationMetadata),
    /// Authenticate as a member of a cloud. Begin receiving `CloudMutated` responses.
    ///
    /// `pubkey_hash, signature_bytes`
    AuthCloud([u8; 32], Vec<u8>),
    /// Get a mutation by index. AuthCloud must be invoked first.
    ///
    /// `mutation_index`
    GetMutation(u64),
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
    /// `latest_known_index, mutation_hash`
    CloudMutated(u64, [u8; 32]),
    /// Retrieve the canonical mutation data for a certain mutation index.
    CloudMutation(MutationMetadata),
    /// keepalive mechanism
    Pong,
}
