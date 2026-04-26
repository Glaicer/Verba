use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AppConfig {
    pub provider: ProviderConfig,
    pub ui: UiConfig,
    pub presets: Vec<Preset>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProviderConfig {
    pub base_url: String,
    pub model_name: String,
    pub timeout_secs: u64,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiConfig {
    pub last_language: String,
    pub last_preset_id: String,
    pub window_width: i32,
    pub window_height: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub instruction: String,
}

impl AppConfig {
    pub fn validate(&mut self) -> Result<()> {
        crate::config::validation::validate(self)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider: ProviderConfig {
                base_url: "https://api.openai.com".to_string(),
                model_name: String::new(),
                timeout_secs: 30,
                temperature: 0.2,
                max_tokens: None,
            },
            ui: UiConfig {
                last_language: "English".to_string(),
                last_preset_id: "precise".to_string(),
                window_width: 960,
                window_height: 640,
            },
            presets: vec![
                Preset {
                    id: "precise".to_string(),
                    name: "Precise".to_string(),
                    instruction: "Translate accurately and preserve the original meaning."
                        .to_string(),
                },
                Preset {
                    id: "natural".to_string(),
                    name: "Natural".to_string(),
                    instruction: "Use natural, fluent wording in the target language.".to_string(),
                },
                Preset {
                    id: "formal".to_string(),
                    name: "Formal".to_string(),
                    instruction: "Use a formal and professional tone.".to_string(),
                },
            ],
        }
    }
}
