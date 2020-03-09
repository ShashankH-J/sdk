//! Provides a basic example provider that utilizes unencrypted PEM
//! files. This is provided as a basic stepping stone to provide
//! further functionality. Note that working with unencrypted PEM is
//! not the best idea.
//!
//! However, there are two options: i) prompt the user per call, as
//! the agent is "stateless" or ii) provide long-running service
//! providers -- such as PGP, ssh-agent.
use crate::crypto_error::{Error, Result};
use crate::principal::Principal;
use crate::provider::{IdentityWallet, Provider};
use crate::types::Signature;

use pem::{encode, Pem};
use ring::signature::Ed25519KeyPair;
use ring::{
    rand,
    signature::{self, KeyPair},
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct BasicProvider {
    path: PathBuf,
}

impl BasicProvider {
    pub fn new(path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            return Err(Error::ProviderFailedToInitialize);
        }
        Ok(Self { path })
    }
}

fn generate(profile_path: &impl AsRef<Path>) -> Result<PathBuf> {
    let rng = rand::SystemRandom::new();
    let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&rng)?;
    // We create a temporary file that gets overwritten every time
    // we create a new provider for now.
    let pem_file = profile_path.as_ref().join("creds.pem");
    fs::write(&pem_file, encode_pem_private_key(&(*pkcs8_bytes.as_ref())))?;

    assert_eq!(
        pem::parse(fs::read(&pem_file)?)?.contents,
        pkcs8_bytes.as_ref()
    );
    Ok(pem_file)
}

struct BasicProviderReady {
    key_pair: Ed25519KeyPair,
}

impl Provider for BasicProvider {
    fn provide(&self) -> Result<Box<dyn IdentityWallet>> {
        let mut dir = fs::read_dir(&self.path)?;
        let name: std::ffi::OsString = "creds.pem".to_owned().into();
        let pem_file = if dir.any(|n| match n {
            Ok(n) => n.file_name() == name,
            Err(_) => false,
        }) {
            self.path.join("creds.pem")
        } else {
            generate(&self.path)?
        };

        let pkcs8_bytes = pem::parse(fs::read(pem_file)?)?.contents;
        let key_pair = signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())?;

        Ok(Box::new(BasicProviderReady { key_pair }))
    }
}

impl IdentityWallet for BasicProviderReady {
    fn sign(&self, msg: &[u8]) -> Result<Signature> {
        let signature = self.key_pair.sign(msg);
        // At this point we shall validate the signature in this first
        // skeleton version.
        let public_key_bytes = self.key_pair.public_key().as_ref();

        let public_key = signature::UnparsedPublicKey::new(&signature::ED25519, public_key_bytes);
        public_key.verify(msg, signature.as_ref())?;
        Ok(Signature {
            signer: self.principal(),
            signature: signature.as_ref().to_vec(),
            public_key: public_key_bytes.to_vec(),
        })
    }
    fn principal(&self) -> Principal {
        Principal::self_authenticating(&self.key_pair)
    }
}

fn encode_pem_private_key(key: &[u8]) -> String {
    let pem = Pem {
        tag: "PRIVATE KEY".to_owned(),
        contents: key.to_vec(),
    };
    encode(&pem)
}