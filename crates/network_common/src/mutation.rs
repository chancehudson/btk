use anyhow::Result;
use ml_dsa::EncodedSignature;
use ml_dsa::EncodedVerifyingKey;
use ml_dsa::MlDsa87;
use ml_dsa::Signature;
use ml_dsa::VerifyingKey;
use ml_dsa::signature::Verifier;
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

    /// Verify that the public_key_hash is correct. Verify that public_key is correct, if present.
    /// Verify the signature.
    pub fn verify(&self, public_key: Vec<u8>) -> Result<()> {
        let pubkey_hash: [u8; 32] = blake3::hash(&public_key).into();
        if pubkey_hash != self.public_key_hash {
            anyhow::bail!("public key hash mismatch");
        }
        if let Some(pubkey) = &self.public_key {
            if pubkey != &public_key {
                anyhow::bail!("mismatched public keys");
            }
        }
        let encoded_vk = EncodedVerifyingKey::<MlDsa87>::try_from(public_key.as_slice())?;
        let vk = VerifyingKey::<MlDsa87>::decode(&encoded_vk);

        let sig_bytes = EncodedSignature::<MlDsa87>::try_from(self.signature.as_slice())?;
        let sig = Signature::<MlDsa87>::decode(&sig_bytes);
        if sig.is_none() {
            anyhow::bail!("failed to parse signature");
        }
        let sig = sig.unwrap();

        vk.verify(self.data.as_slice(), &sig)
            .map_err(|err| anyhow::anyhow!("signature verification failed: {:?}", err))?;

        Ok(())
    }
}
