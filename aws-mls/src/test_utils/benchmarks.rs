use aws_mls_codec::MlsEncode;
use aws_mls_core::protocol_version::ProtocolVersion;

use crate::{
    cipher_suite::CipherSuite,
    client_builder::{
        BaseConfig, MlsConfig, Preferences, WithCryptoProvider, WithIdentityProvider,
    },
    group::{framing::MLSMessage, Group},
    identity::basic::BasicIdentityProvider,
    test_utils::{generate_basic_client, get_test_groups},
};

#[cfg(awslc)]
pub use aws_mls_crypto_awslc::AwsLcCryptoProvider as MlsCryptoProvider;
#[cfg(not(any(awslc, rustcrypto)))]
pub use aws_mls_crypto_openssl::OpensslCryptoProvider as MlsCryptoProvider;
#[cfg(rustcrypto)]
pub use aws_mls_crypto_rustcrypto::RustCryptoProvider as MlsCryptoProvider;

pub type TestClientConfig =
    WithIdentityProvider<BasicIdentityProvider, WithCryptoProvider<MlsCryptoProvider, BaseConfig>>;

macro_rules! load_test_case_mls {
    ($name:ident, $generate:expr) => {
        load_test_case_mls!($name, $generate, to_vec_pretty)
    };
    ($name:ident, $generate:expr, $to_json:ident) => {{
        #[cfg(any(target_arch = "wasm32", not(feature = "std")))]
        {
            // Do not remove `async`! (The goal of this line is to remove warnings
            // about `$generate` not being used. Actually calling it will make tests fail.)
            let _ = async { $generate };

            aws_mls_codec::MlsDecode::mls_decode(&mut &include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/",
                stringify!($name),
                ".mls"
            )))
            .unwrap()
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "std"))]
        {
            let path = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/",
                stringify!($name),
                ".mls"
            );

            if !std::path::Path::new(path).exists() {
                std::fs::write(path, $generate.mls_encode_to_vec().unwrap()).unwrap();
            }

            aws_mls_codec::MlsDecode::mls_decode(&mut std::fs::read(path).unwrap().as_slice())
                .unwrap()
        }
    }};
}

#[cfg_attr(not(mls_build_async), maybe_async::must_be_sync)]
async fn generate_test_cases(cs: CipherSuite) -> Vec<MLSMessage> {
    let mut cases = Vec::new();

    for size in [16, 64, 128] {
        let preferences = Preferences::default().with_ratchet_tree_extension(true);

        let group = get_test_groups(
            ProtocolVersion::MLS_10,
            cs,
            size,
            &preferences,
            &MlsCryptoProvider::new(),
        )
        .await
        .pop()
        .unwrap();

        let group_info = group
            .group_info_message_allowing_ext_commit()
            .await
            .unwrap();

        cases.push(group_info)
    }

    cases
}

#[derive(Clone)]
pub struct GroupStates<C: MlsConfig> {
    pub sender: Group<C>,
    pub receiver: Group<C>,
}

#[cfg(mls_build_async)]
pub fn load_group_states(cs: CipherSuite) -> Vec<GroupStates<impl MlsConfig>> {
    let group_info = load_test_case_mls!(group_state, block_on(generate_test_cases(cs)), to_vec);
    join_group(cs, group_info)
}

#[cfg(not(mls_build_async))]
pub fn load_group_states(cs: CipherSuite) -> Vec<GroupStates<impl MlsConfig>> {
    let group_infos: Vec<MLSMessage> =
        load_test_case_mls!(group_state, generate_test_cases(cs), to_vec);

    group_infos
        .into_iter()
        .map(|info| join_group(cs, info))
        .collect()
}

#[cfg_attr(not(mls_build_async), maybe_async::must_be_sync)]
pub async fn join_group(cs: CipherSuite, group_info: MLSMessage) -> GroupStates<impl MlsConfig> {
    let client = generate_basic_client(
        cs,
        ProtocolVersion::MLS_10,
        99999999999,
        &Preferences::default().with_ratchet_tree_extension(true),
        &MlsCryptoProvider::new(),
    );

    let mut sender = client.commit_external(group_info).await.unwrap().0;

    let client = generate_basic_client(
        cs,
        ProtocolVersion::MLS_10,
        99999999998,
        &Preferences::default(),
        &MlsCryptoProvider::new(),
    );

    let group_info = sender
        .group_info_message_allowing_ext_commit()
        .await
        .unwrap();

    let (receiver, commit) = client.commit_external(group_info).await.unwrap();
    sender.process_incoming_message(commit).await.unwrap();

    GroupStates { sender, receiver }
}
