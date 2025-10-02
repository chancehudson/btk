# BTK

A local first productivity suite. Exposes a nosql api to a journaled database that is synchronized between potentially untrusted peers. Each database is referred to as an "encrypted cloud".

## Cryptographic structure

Each cloud has a 32 byte private key (`[u8; 32]`). From this key we derive an ML-DSA keypair. The hash of the public key is the cloud identifier.

Each encrypted change to the cloud is called a "mutation". Each change is encrypted with a key that is `H(private_key, index, salt)`. `index` is the index of the mutation being applied, and `salt` is 32 random bytes.

Each mutation includes a signature of the encrypted data.

Each mutation contains a journal entry, which forms a hashchain for a given cloud.

## To run

Clone the repo and run the following:

`cargo run --bin=btk_client --release`

