use std::fs;

use verba::{
    config::{AppConfig, ConfigStore},
    gui::{
        preset_editor::PresetEditorModel,
        settings_dialog::{apply_settings, SettingsDraft},
    },
};

mod support {
    use std::sync::atomic::{AtomicBool, Ordering};

    use verba::secrets::{SecretFuture, SecretStore};

    #[derive(Default)]
    pub struct NoopSecretStore {
        pub cleared: AtomicBool,
    }

    impl SecretStore for NoopSecretStore {
        fn get_api_key(&self) -> SecretFuture<'_, Option<String>> {
            Box::pin(async { Ok(None) })
        }

        fn set_api_key<'a>(&'a self, _value: &'a str) -> SecretFuture<'a, ()> {
            Box::pin(async { Ok(()) })
        }

        fn clear_api_key(&self) -> SecretFuture<'_, ()> {
            Box::pin(async move {
                self.cleared.store(true, Ordering::SeqCst);
                Ok(())
            })
        }
    }
}

#[test]
fn preset_editor_should_add_preset_with_generated_slug_id() {
    let mut editor = PresetEditorModel::new(AppConfig::default().presets);

    editor
        .add_preset("Quick Notes", "Use compact wording.")
        .expect("preset should be added");

    let preset = editor.presets().last().expect("new preset should exist");
    assert_eq!(preset.id, "quick-notes");
}

#[test]
fn preset_editor_should_delete_preset() {
    let mut editor = PresetEditorModel::new(AppConfig::default().presets);

    let deleted = editor.delete_preset("natural").expect("delete should work");

    assert!(deleted);
    assert!(editor.presets().iter().all(|preset| preset.id != "natural"));
}

#[test]
fn preset_editor_should_not_delete_last_preset() {
    let mut presets = AppConfig::default().presets;
    let mut editor = PresetEditorModel::new(vec![presets.remove(0)]);

    let err = editor
        .delete_preset("precise")
        .expect_err("last preset must not be deleted");

    assert!(err.to_string().contains("last preset cannot be deleted"));
}

#[test]
fn preset_editor_should_reject_invalid_presets() {
    let mut editor = PresetEditorModel::new(AppConfig::default().presets);
    editor
        .add_preset("Precise", "Duplicate name.")
        .expect("add does not validate full set");

    let err = editor.validate().expect_err("duplicate names must fail");

    assert!(err.to_string().contains("duplicate preset name"));
}

#[test]
fn preset_editor_should_reject_empty_fields() {
    let mut editor = PresetEditorModel::new(AppConfig::default().presets);

    let err = editor
        .add_preset("", "Instruction")
        .expect_err("empty name must fail");
    assert!(err.to_string().contains("preset name is required"));

    let err = editor
        .add_preset("Short", "")
        .expect_err("empty instruction must fail");
    assert!(err.to_string().contains("preset instruction is required"));
}

#[tokio::test]
async fn settings_save_should_persist_staged_presets() {
    let dir = std::env::temp_dir().join(format!("verba-preset-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).expect("test temp dir should be created");
    let store = ConfigStore::new(dir.join("config.toml"));
    let secrets = support::NoopSecretStore::default();
    let mut draft = SettingsDraft::from_config(&AppConfig::default());
    draft.model_name = "test-model".to_string();
    draft.presets = PresetEditorModel::new(AppConfig::default().presets)
        .with_added_preset("Developer", "Preserve code terms.")
        .expect("preset should be added")
        .into_presets();

    apply_settings(&store, &secrets, AppConfig::default(), draft)
        .await
        .expect("settings should save");

    let saved = fs::read_to_string(store.path()).expect("config should be saved");
    assert!(saved.contains("name = \"Developer\""));
}
