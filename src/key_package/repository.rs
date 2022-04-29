use crate::key_package::{KeyPackageError, KeyPackageGeneration, KeyPackageRef};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub trait KeyPackageRepository {
    type Error: std::error::Error + Send + Sync + 'static;

    fn insert(&mut self, key_pkg_gen: KeyPackageGeneration) -> Result<(), Self::Error>;
    fn get(&self, key_pkg: &KeyPackageRef) -> Result<Option<KeyPackageGeneration>, Self::Error>;
}

#[derive(Clone, Default, Debug)]
pub struct InMemoryKeyPackageRepository {
    inner: Arc<Mutex<HashMap<KeyPackageRef, KeyPackageGeneration>>>,
}

impl InMemoryKeyPackageRepository {
    pub fn insert(&self, key_pkg_gen: KeyPackageGeneration) -> Result<(), KeyPackageError> {
        self.inner
            .lock()
            .unwrap()
            .insert(key_pkg_gen.key_package.to_reference()?, key_pkg_gen);
        Ok(())
    }

    pub fn get(&self, r: &KeyPackageRef) -> Option<KeyPackageGeneration> {
        self.inner.lock().unwrap().get(r).cloned()
    }
}

impl KeyPackageRepository for InMemoryKeyPackageRepository {
    type Error = KeyPackageError;

    fn insert(&mut self, key_pkg_gen: KeyPackageGeneration) -> Result<(), Self::Error> {
        (*self).insert(key_pkg_gen)
    }

    fn get(&self, key_pkg: &KeyPackageRef) -> Result<Option<KeyPackageGeneration>, Self::Error> {
        Ok(self.get(key_pkg))
    }
}