#![allow(clippy::nonstandard_macro_braces)]
#![allow(clippy::enum_variant_names)]

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg(test)]
macro_rules! hex {
    ($input:literal) => {
        hex::decode($input).expect("invalid hex value")
    };
}

#[cfg(any(test, feature = "benchmark"))]
macro_rules! load_test_cases {
    ($name:ident, $generate:expr) => {
        load_test_cases!($name, $generate, to_vec_pretty)
    };
    ($name:ident, $generate:expr, $to_json:ident) => {{
        #[cfg(target_arch = "wasm32")]
        {
            let _ = $generate;
            serde_json::from_slice(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/",
                stringify!($name),
                ".json"
            )))
            .unwrap()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/",
                stringify!($name),
                ".json"
            );
            if !std::path::Path::new(path).exists() {
                std::fs::write(path, serde_json::$to_json(&$generate()).unwrap()).unwrap();
            }
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
        }
    }};
}

#[macro_use]
pub mod cipher_suite;
pub mod client;
pub mod client_config;
pub mod credential;
mod epoch;
pub mod extension;
mod external_client;
mod external_client_config;
mod external_session;
mod hash_reference;
pub mod key_package;
pub mod keychain;
pub mod message;
mod protocol_version;
mod psk;
mod serde_utils;
pub mod session;
pub mod signer;
pub mod signing_identity;
pub mod x509;

#[cfg(feature = "benchmark")]
pub mod bench_utils;

#[cfg(feature = "benchmark")]
pub mod tls;

#[cfg(feature = "benchmark")]
pub mod tree_kem;

#[cfg(feature = "benchmark")]
pub mod group;

#[cfg(not(feature = "benchmark"))]
mod tls;

#[cfg(not(feature = "benchmark"))]
mod tree_kem;

#[cfg(not(feature = "benchmark"))]
mod group;

pub use epoch::{Epoch, EpochRepository, InMemoryEpochRepository};
pub use external_client::ExternalClient;
pub use external_client_config::{ExternalClientConfig, InMemoryExternalClientConfig};
pub use external_session::ExternalSession;
pub use ferriscrypt;
pub use group::{
    proposal::{AddProposal, Proposal, RemoveProposal, UpdateProposal},
    BoxedProposalFilter, ConfirmedTranscriptHash, GroupContext, PassThroughProposalFilter,
    ProposalBundle, ProposalFilter, ProposalFilterError,
};
pub use protocol_version::ProtocolVersion;
pub use tls_codec;

pub mod time;
