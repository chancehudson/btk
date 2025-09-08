use serde::Deserialize;
use serde::Serialize;

/// Public data for a mutation to an encrypted cloud.
/// Used to ensure consistency among synchronized devices.
///
/// Data is encypted with key H(H(private_key), index, salt), and the encrypted bytes are signed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mutation {
    /// Global counter of mutations
    pub index: u64,
    /// Encrypted mutation/diff/action
    pub data: Vec<u8>,
    /// Variable length signature, impl defined algo
    pub signature: Vec<u8>,
    /// 32 byte public key hash, impl defined algo
    pub public_key_hash: [u8; 32],
    /// Optional full public key. This must be provided if `index == 0` as the encrypted cloud is
    /// being created.
    pub public_key: Option<Vec<u8>>,
    /// Salt used to compute a distinct encryption key for the mutation. This is necessary to
    /// prevent cases where two changes for the same index are created and broadcasted, but encrypted
    /// with the same key+nonce. Such a case would leak the key in most symmetric
    /// encryption constructions.
    pub salt: [u8; 32],
    /// Optionally provide the encryption key for the mutation. Setting this value makes the
    /// mutation irreversibly public.
    pub mutation_key: Option<[u8; 32]>,
}

impl Mutation {
    pub fn hash() -> [u8; 32] {
        unimplemented!();
    }
}
