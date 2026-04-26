use std::fs;

use clap::Parser;
use verba::{
    cli::{Cli, Command},
    config::{AppConfig, ConfigStore, Preset},
};

#[test]
fn cli_should_parse_supported_commands() {
    assert!(matches!(
        Cli::parse_from(["verba", "daemon"]).command,
        Command::Daemon
    ));
    assert!(matches!(
        Cli::parse_from(["verba", "toggle"]).command,
        Command::Toggle
    ));
    assert!(matches!(
        Cli::parse_from(["verba", "show"]).command,
        Command::Show
    ));
    assert!(matches!(
        Cli::parse_from(["verba", "hide"]).command,
        Command::Hide
    ));
    assert!(matches!(
        Cli::parse_from(["verba", "settings"]).command,
        Command::Settings
    ));
    assert!(matches!(
        Cli::parse_from(["verba", "quit"]).command,
        Command::Quit
    ));
}

#[test]
fn default_config_should_include_required_presets_without_api_key() {
    let config = AppConfig::default();

    let names: Vec<_> = config
        .presets
        .iter()
        .map(|preset| preset.name.as_str())
        .collect();
    assert_eq!(names, ["Precise", "Natural", "Formal"]);

    let toml = toml::to_string(&config).expect("default config should serialize");
    assert!(!toml.to_ascii_lowercase().contains("api_key"));
}

#[test]
fn config_validation_should_reject_duplicate_preset_names() {
    let mut config = AppConfig::default();
    config.presets.push(Preset {
        id: "another-precise".to_string(),
        name: "Precise".to_string(),
        instruction: "Use exact wording.".to_string(),
    });

    let err = config
        .validate()
        .expect_err("duplicate preset names must be rejected");
    assert!(err.to_string().contains("duplicate preset name"));
}

#[test]
fn config_validation_should_normalize_provider_base_url() {
    let mut config = AppConfig::default();
    config.provider.base_url = "https://api.openai.com/".to_string();

    config.validate().expect("base URL should be valid");

    assert_eq!(config.provider.base_url, "https://api.openai.com");
}

#[test]
fn config_validation_should_reject_chat_completions_endpoint_as_base_url() {
    let mut config = AppConfig::default();
    config.provider.base_url = "https://api.openai.com/v1/chat/completions".to_string();

    let err = config
        .validate()
        .expect_err("full endpoint must be rejected");

    assert!(err.to_string().contains("/v1/chat/completions"));
}

#[test]
fn config_store_should_create_default_config_when_missing() {
    let dir = tempfile_dir();
    let path = dir.join("config.toml");
    let store = ConfigStore::new(path.clone());

    let config = store
        .load_or_create()
        .expect("missing config should be created");

    assert_eq!(config.presets.len(), 3);
    assert!(path.exists());
    assert!(!fs::read_to_string(path).unwrap().contains("api_key"));
}

fn tempfile_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("verba-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).expect("test temp dir should be created");
    dir
}
