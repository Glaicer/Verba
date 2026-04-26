pub mod secret_service;

use std::{future::Future, pin::Pin};

use crate::error::Result;

pub type SecretFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

pub trait SecretStore: Send + Sync {
    fn get_api_key(&self) -> SecretFuture<'_, Option<String>>;
    fn set_api_key<'a>(&'a self, value: &'a str) -> SecretFuture<'a, ()>;
    fn clear_api_key(&self) -> SecretFuture<'_, ()>;
}
