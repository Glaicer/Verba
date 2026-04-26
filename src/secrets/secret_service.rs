use std::{collections::HashMap, str};

use secret_service::{EncryptionType, SecretService};

use crate::{
    error::{Result, VerbaError},
    secrets::{SecretFuture, SecretStore},
};

const LABEL: &str = "Verba API Key";
const CONTENT_TYPE: &str = "text/plain";
const APPLICATION_ATTR: &str = "application";
const APPLICATION_VALUE: &str = "verba";
const KIND_ATTR: &str = "kind";
const KIND_VALUE: &str = "api-key";

#[derive(Clone, Debug, Default)]
pub struct SecretServiceStore;

impl SecretServiceStore {
    pub fn new() -> Self {
        Self
    }
}

impl SecretStore for SecretServiceStore {
    fn get_api_key(&self) -> SecretFuture<'_, Option<String>> {
        Box::pin(async move {
            let ss = connect().await?;
            let Some(item) = find_item(&ss).await? else {
                return Ok(None);
            };

            let secret = item
                .get_secret()
                .await
                .map_err(|err| VerbaError::Secret(unavailable_message(err)))?;
            let value = str::from_utf8(&secret)
                .map_err(|_| VerbaError::Secret("stored API key is not valid UTF-8".to_string()))?;
            Ok(Some(value.to_string()))
        })
    }

    fn set_api_key<'a>(&'a self, value: &'a str) -> SecretFuture<'a, ()> {
        Box::pin(async move {
            if value.trim().is_empty() {
                return self.clear_api_key().await;
            }

            let ss = connect().await?;
            let collection = ss
                .get_default_collection()
                .await
                .map_err(|err| VerbaError::Secret(unavailable_message(err)))?;
            collection
                .create_item(LABEL, attributes(), value.as_bytes(), true, CONTENT_TYPE)
                .await
                .map_err(|err| VerbaError::Secret(unavailable_message(err)))?;
            Ok(())
        })
    }

    fn clear_api_key(&self) -> SecretFuture<'_, ()> {
        Box::pin(async move {
            let ss = connect().await?;
            if let Some(item) = find_item(&ss).await? {
                item.delete()
                    .await
                    .map_err(|err| VerbaError::Secret(unavailable_message(err)))?;
            }
            Ok(())
        })
    }
}

async fn connect<'a>() -> Result<SecretService<'a>> {
    SecretService::connect(EncryptionType::Dh)
        .await
        .map_err(|err| VerbaError::Secret(unavailable_message(err)))
}

async fn find_item<'a>(ss: &'a SecretService<'a>) -> Result<Option<secret_service::Item<'a>>> {
    let items = ss
        .search_items(attributes())
        .await
        .map_err(|err| VerbaError::Secret(unavailable_message(err)))?;

    if let Some(item) = items.unlocked.into_iter().next() {
        return Ok(Some(item));
    }

    let Some(item) = items.locked.into_iter().next() else {
        return Ok(None);
    };
    item.unlock()
        .await
        .map_err(|err| VerbaError::Secret(unavailable_message(err)))?;
    Ok(Some(item))
}

fn attributes() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        (APPLICATION_ATTR, APPLICATION_VALUE),
        (KIND_ATTR, KIND_VALUE),
    ])
}

fn unavailable_message(err: impl std::fmt::Display) -> String {
    format!("Secret Service is unavailable or rejected the request: {err}")
}
