use std::{
    fs,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use verba::{
    config::{AppConfig, ConfigStore},
    error::VerbaError,
    gui::settings_dialog::{apply_settings, ApiKeyEdit, SettingsDraft},
    secrets::{SecretFuture, SecretStore},
};

#[test]
fn settings_should_reject_invalid_base_url() {
    let mut draft = valid_draft();
    draft.base_url = "not a url".to_string();

    let err = draft
        .validated_config(AppConfig::default())
        .expect_err("invalid URL must fail");

    assert!(err.to_string().contains("invalid provider base URL"));
}

#[test]
fn settings_should_reject_chat_completions_endpoint_as_base_url() {
    let mut draft = valid_draft();
    draft.base_url = "https://api.openai.com/v1/chat/completions".to_string();

    let err = draft
        .validated_config(AppConfig::default())
        .expect_err("full endpoint must fail");

    assert!(err.to_string().contains("/v1/chat/completions"));
}

#[test]
fn settings_should_reject_empty_model_name() {
    let mut draft = valid_draft();
    draft.model_name = "  ".to_string();

    let err = draft
        .validated_config(AppConfig::default())
        .expect_err("empty model must fail");

    assert!(err.to_string().contains("model name is required"));
}

#[tokio::test]
async fn settings_should_save_config_without_api_key() {
    let dir = tempfile_dir();
    let store = ConfigStore::new(dir.join("config.toml"));
    let secrets = RecordingSecretStore::default();

    apply_settings(
        &store,
        &secrets,
        AppConfig::default(),
        valid_draft().with_api_key(ApiKeyEdit::Replace("sk-secret".to_string())),
    )
    .await
    .expect("settings should save");

    let saved = fs::read_to_string(store.path()).expect("config should be written");
    assert!(saved.contains("model_name = \"test-model\""));
    assert!(!saved.contains("sk-secret"));
    assert_eq!(secrets.last_set(), Some("sk-secret".to_string()));
}

#[tokio::test]
async fn settings_should_not_write_config_when_secret_service_save_fails() {
    let dir = tempfile_dir();
    let path = dir.join("config.toml");
    let store = ConfigStore::new(path.clone());
    let secrets = RecordingSecretStore::failing();

    let err = apply_settings(
        &store,
        &secrets,
        AppConfig::default(),
        valid_draft().with_api_key(ApiKeyEdit::Replace("sk-secret".to_string())),
    )
    .await
    .expect_err("secret failure must block save");

    assert!(err.to_string().contains("Secret Service error"));
    assert!(!path.exists());
}

fn valid_draft() -> SettingsDraft {
    SettingsDraft {
        base_url: "https://api.openai.com/".to_string(),
        model_name: "test-model".to_string(),
        api_key: ApiKeyEdit::Unchanged,
        presets: AppConfig::default().presets,
    }
}

fn tempfile_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("verba-settings-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).expect("test temp dir should be created");
    dir
}

#[derive(Default)]
struct RecordingSecretStore {
    fail: AtomicBool,
    last_set: Arc<Mutex<Option<String>>>,
    cleared: AtomicBool,
}

impl RecordingSecretStore {
    fn failing() -> Self {
        Self {
            fail: AtomicBool::new(true),
            last_set: Arc::default(),
            cleared: AtomicBool::new(false),
        }
    }

    fn last_set(&self) -> Option<String> {
        self.last_set
            .lock()
            .expect("secret test mutex should not be poisoned")
            .clone()
    }
}

impl SecretStore for RecordingSecretStore {
    fn get_api_key(&self) -> SecretFuture<'_, Option<String>> {
        Box::pin(async { Ok(None) })
    }

    fn set_api_key<'a>(&'a self, value: &'a str) -> SecretFuture<'a, ()> {
        Box::pin(async move {
            if self.fail.load(Ordering::SeqCst) {
                return Err(VerbaError::Secret(
                    "mock Secret Service failure".to_string(),
                ));
            }

            *self
                .last_set
                .lock()
                .expect("secret test mutex should not be poisoned") = Some(value.to_string());
            Ok(())
        })
    }

    fn clear_api_key(&self) -> SecretFuture<'_, ()> {
        Box::pin(async move {
            self.cleared.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
}
