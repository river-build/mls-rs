use std::ops::Deref;

use aws_mls_core::crypto::{CipherSuite, SignaturePublicKey, SignatureSecretKey};
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Private, Public},
};
use thiserror::Error;

use crate::ec::{
    curve_from_private_key, curve_from_public_key, generate_keypair, private_key_bytes_to_public,
    private_key_from_bytes, private_key_from_der, private_key_to_bytes, pub_key_from_uncompressed,
    pub_key_to_uncompressed, public_key_from_der, Curve, EcError,
};

#[derive(Debug, Error)]
pub enum EcSignerError {
    #[error(transparent)]
    OpensslError(#[from] openssl::error::ErrorStack),
    #[error(transparent)]
    EcError(#[from] EcError),
    #[error("invalid signature")]
    InvalidSignature,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub struct EcSigner(Curve);

impl Deref for EcSigner {
    type Target = Curve;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl EcSigner {
    pub fn new(cipher_suite: CipherSuite) -> Result<Self, EcSignerError> {
        Curve::from_ciphersuite(cipher_suite, true)
            .map(Self)
            .map_err(Into::into)
    }

    pub fn signature_key_generate(
        &self,
    ) -> Result<(SignatureSecretKey, SignaturePublicKey), EcSignerError> {
        let key_pair = generate_keypair(self.0)?;
        Ok((key_pair.secret.into(), key_pair.public.into()))
    }

    pub fn signature_key_import_der_public(
        &self,
        der_data: &[u8],
    ) -> Result<SignaturePublicKey, EcError> {
        let key = public_key_from_der(der_data)?;

        curve_from_public_key(&key)
            .filter(|&c| c == self.0)
            .ok_or(EcError::InvalidKeyBytes)?;

        Ok(pub_key_to_uncompressed(&key)?.into())
    }

    pub fn signature_key_import_der_private(
        &self,
        der_data: &[u8],
    ) -> Result<SignatureSecretKey, EcError> {
        let key = private_key_from_der(der_data)?;

        curve_from_private_key(&key)
            .filter(|&c| c == self.0)
            .ok_or(EcError::InvalidKeyBytes)?;

        Ok(private_key_to_bytes(&key)?.into())
    }

    pub fn signature_key_derive_public(
        &self,
        secret_key: &SignatureSecretKey,
    ) -> Result<SignaturePublicKey, EcSignerError> {
        Ok(private_key_bytes_to_public(secret_key, self.0)?.into())
    }

    pub(crate) fn pkey_from_secret_key(
        &self,
        key: &SignatureSecretKey,
    ) -> Result<PKey<Private>, EcSignerError> {
        private_key_from_bytes(key, self.0).map_err(Into::into)
    }

    pub(crate) fn pkey_from_public_key(
        &self,
        key: &SignaturePublicKey,
    ) -> Result<PKey<Public>, EcSignerError> {
        pub_key_from_uncompressed(key, self.0).map_err(Into::into)
    }

    pub fn sign(
        &self,
        secret_key: &SignatureSecretKey,
        data: &[u8],
    ) -> Result<Vec<u8>, EcSignerError> {
        let secret_key = private_key_from_bytes(secret_key, self.0)?;

        let mut signer = match self.message_digest() {
            Some(md) => openssl::sign::Signer::new(md, &secret_key),
            None => openssl::sign::Signer::new_without_digest(&secret_key),
        }?;

        Ok(signer.sign_oneshot_to_vec(data)?)
    }

    pub fn verify(
        &self,
        public_key: &SignaturePublicKey,
        signature: &[u8],
        data: &[u8],
    ) -> Result<(), EcSignerError> {
        let public_key = pub_key_from_uncompressed(public_key, self.0)?;

        let mut verifier = match self.message_digest() {
            Some(md) => openssl::sign::Verifier::new(md, &public_key),
            None => openssl::sign::Verifier::new_without_digest(&public_key),
        }?;

        verifier
            .verify_oneshot(signature, data)?
            .then_some(())
            .ok_or(EcSignerError::InvalidSignature)
    }

    pub(crate) fn message_digest(&self) -> Option<MessageDigest> {
        match self.0 {
            Curve::P256 => Some(MessageDigest::sha256()),
            Curve::P384 => Some(MessageDigest::sha384()),
            Curve::P521 => Some(MessageDigest::sha512()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use assert_matches::assert_matches;
    use aws_mls_core::crypto::CipherSuite;

    use crate::{
        ec::{
            test_utils::{
                get_test_public_keys, get_test_public_keys_der, get_test_secret_keys,
                get_test_secret_keys_der, TestKeys,
            },
            Curve,
        },
        ec_signer::{EcSigner, EcSignerError},
    };

    const TEST_INPUT: &[u8] = b"Hello World!";

    #[test]
    fn test_signatures() {
        CipherSuite::all().for_each(test_signature);
    }

    fn test_signature(cipher_suite: CipherSuite) {
        println!("Testing signatures for cipher suite: {cipher_suite:?}");

        let public_key = get_test_public_keys().get_key(cipher_suite, true).into();
        let secret_key = get_test_secret_keys().get_key(cipher_suite, true);
        let ec_signer = EcSigner::new(cipher_suite).unwrap();

        assert_eq!(ec_signer.secret_key_size(), secret_key.len());

        let sig = ec_signer.sign(&secret_key.into(), TEST_INPUT).unwrap();

        ec_signer.verify(&public_key, &sig, TEST_INPUT).unwrap();

        let other_public_key = ec_signer.signature_key_generate().unwrap().1;

        assert_matches!(
            ec_signer.verify(&other_public_key, &sig, TEST_INPUT),
            Err(EcSignerError::InvalidSignature)
        );

        assert_matches!(
            ec_signer.verify(&public_key, &sig, &[TEST_INPUT, &[0]].concat()),
            Err(EcSignerError::InvalidSignature)
        );
    }

    #[test]
    fn import_der_public() {
        let keys = get_test_public_keys();
        let der_keys = get_test_public_keys_der();

        let convert = |keys: &TestKeys, curve: Curve| {
            EcSigner(curve)
                .signature_key_import_der_public(&keys.get_key_from_curve(curve))
                .unwrap()
        };

        let converted = TestKeys {
            p256: convert(&der_keys, Curve::P256).to_vec(),
            p384: convert(&der_keys, Curve::P384).to_vec(),
            p521: convert(&der_keys, Curve::P521).to_vec(),
            x25519: convert(&der_keys, Curve::X25519).to_vec(),
            ed25519: convert(&der_keys, Curve::Ed25519).to_vec(),
            x448: convert(&der_keys, Curve::X448).to_vec(),
            ed448: convert(&der_keys, Curve::Ed448).to_vec(),
        };

        assert_eq!(keys, converted);
    }

    #[test]
    fn import_der_private() {
        let keys = get_test_secret_keys();
        let der_keys = get_test_secret_keys_der();

        let convert = |keys: &TestKeys, curve: Curve| {
            EcSigner(curve)
                .signature_key_import_der_private(&keys.get_key_from_curve(curve))
                .unwrap()
        };

        let converted = TestKeys {
            p256: convert(&der_keys, Curve::P256).to_vec(),
            p384: convert(&der_keys, Curve::P384).to_vec(),
            p521: convert(&der_keys, Curve::P521).to_vec(),
            x25519: convert(&der_keys, Curve::X25519).to_vec(),
            ed25519: convert(&der_keys, Curve::Ed25519).to_vec(),
            x448: convert(&der_keys, Curve::X448).to_vec(),
            ed448: convert(&der_keys, Curve::Ed448).to_vec(),
        };

        assert_eq!(keys, converted);
    }
}
