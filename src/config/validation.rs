use std::collections::HashSet;

use url::Url;

use crate::{
    config::schema::AppConfig,
    error::{Result, VerbaError},
};

const CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";

pub fn validate(config: &mut AppConfig) -> Result<()> {
    validate_provider(config)?;
    validate_presets(config)?;
    Ok(())
}

fn validate_provider(config: &mut AppConfig) -> Result<()> {
    let parsed = Url::parse(config.provider.base_url.trim())
        .map_err(|err| VerbaError::InvalidBaseUrl(err.to_string()))?;

    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(VerbaError::InvalidBaseUrl(format!(
                "unsupported URL scheme `{scheme}`"
            )));
        }
    }

    let normalized_path = parsed.path().trim_end_matches('/');
    if normalized_path.eq_ignore_ascii_case(CHAT_COMPLETIONS_PATH) {
        return Err(VerbaError::InvalidBaseUrl(format!(
            "base_url must not include {CHAT_COMPLETIONS_PATH}"
        )));
    }

    if config.provider.timeout_secs == 0 {
        return Err(VerbaError::Config(
            "provider.timeout_secs must be positive".to_string(),
        ));
    }

    if !(0.0..=2.0).contains(&config.provider.temperature) {
        return Err(VerbaError::Config(
            "provider.temperature must be between 0.0 and 2.0".to_string(),
        ));
    }

    config.provider.base_url = normalize_base_url(parsed);
    Ok(())
}

fn normalize_base_url(mut url: Url) -> String {
    url.set_fragment(None);
    url.set_query(None);

    let mut text = url.to_string();
    while text.ends_with('/') {
        text.pop();
    }
    text
}

fn validate_presets(config: &AppConfig) -> Result<()> {
    if config.presets.is_empty() {
        return Err(VerbaError::Config(
            "at least one preset is required".to_string(),
        ));
    }

    let mut names = HashSet::new();
    for preset in &config.presets {
        if preset.id.trim().is_empty() {
            return Err(VerbaError::Config("preset id is required".to_string()));
        }
        if preset.name.trim().is_empty() {
            return Err(VerbaError::Config("preset name is required".to_string()));
        }
        let normalized_name = preset.name.trim().to_ascii_lowercase();
        if !names.insert(normalized_name) {
            return Err(VerbaError::Config(format!(
                "duplicate preset name `{}`",
                preset.name
            )));
        }
        if preset.instruction.trim().is_empty() {
            return Err(VerbaError::Config(
                "preset instruction is required".to_string(),
            ));
        }
    }

    Ok(())
}
