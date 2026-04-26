use std::sync::{Arc, Mutex};

use url::Url;

use crate::{
    config::AppConfig,
    llm::{
        client::LlmClient,
        errors::{LlmError, LlmErrorKind},
    },
    notify::{Notifier, Urgency},
    secrets::SecretStore,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppState {
    Hidden,
    VisibleIdle,
    VisibleTranslating,
    HiddenTranslating,
    SettingsOpen,
    Exiting,
    CancellingThenExit,
}

#[derive(Debug)]
struct RuntimeState {
    state: AppState,
    busy: bool,
    current_preset: String,
    config: AppConfig,
}

#[derive(Clone, Debug)]
pub struct AppRuntime {
    inner: Arc<Mutex<RuntimeState>>,
}

impl AppRuntime {
    pub fn new(current_preset: impl Into<String>) -> Self {
        let mut config = AppConfig::default();
        config.ui.last_preset_id = current_preset.into();
        Self::from_config(config)
    }

    pub fn from_config(config: AppConfig) -> Self {
        let current_preset = config.ui.last_preset_id.clone();
        Self {
            inner: Arc::new(Mutex::new(RuntimeState {
                state: AppState::Hidden,
                busy: false,
                current_preset,
                config,
            })),
        }
    }

    pub fn toggle_main_window(&self) {
        let mut inner = self
            .inner
            .lock()
            .expect("runtime mutex should not be poisoned");
        inner.state = match inner.state {
            AppState::Hidden => AppState::VisibleIdle,
            AppState::HiddenTranslating => AppState::VisibleTranslating,
            AppState::VisibleIdle => AppState::Hidden,
            AppState::VisibleTranslating => AppState::HiddenTranslating,
            AppState::SettingsOpen => AppState::Hidden,
            AppState::Exiting | AppState::CancellingThenExit => inner.state,
        };
    }

    pub fn show_main_window(&self) {
        let mut inner = self
            .inner
            .lock()
            .expect("runtime mutex should not be poisoned");
        inner.state = match inner.state {
            AppState::Hidden => AppState::VisibleIdle,
            AppState::HiddenTranslating => AppState::VisibleTranslating,
            AppState::Exiting | AppState::CancellingThenExit => inner.state,
            _ => inner.state,
        };
    }

    pub fn hide_main_window(&self) {
        let mut inner = self
            .inner
            .lock()
            .expect("runtime mutex should not be poisoned");
        inner.state = match inner.state {
            AppState::VisibleIdle | AppState::SettingsOpen => AppState::Hidden,
            AppState::VisibleTranslating => AppState::HiddenTranslating,
            AppState::Exiting | AppState::CancellingThenExit => inner.state,
            _ => inner.state,
        };
    }

    pub fn open_settings(&self) {
        let mut inner = self
            .inner
            .lock()
            .expect("runtime mutex should not be poisoned");
        if !matches!(
            inner.state,
            AppState::Exiting | AppState::CancellingThenExit
        ) {
            inner.state = AppState::SettingsOpen;
        }
    }

    pub fn quit(&self) {
        let mut inner = self
            .inner
            .lock()
            .expect("runtime mutex should not be poisoned");
        inner.state = if inner.busy {
            AppState::CancellingThenExit
        } else {
            AppState::Exiting
        };
    }

    pub fn reload_config(&self) {
        // Config reload will be wired once the full ConfigStore-owned runtime exists.
    }

    pub fn update_config(&self, config: AppConfig) {
        let mut inner = self
            .inner
            .lock()
            .expect("runtime mutex should not be poisoned");
        inner.current_preset = config.ui.last_preset_id.clone();
        inner.config = config;
    }

    pub fn translate(&self) {
        let mut inner = self
            .inner
            .lock()
            .expect("runtime mutex should not be poisoned");
        inner.busy = true;
        inner.state = match inner.state {
            AppState::Hidden => AppState::HiddenTranslating,
            AppState::Exiting | AppState::CancellingThenExit => inner.state,
            _ => AppState::VisibleTranslating,
        };
    }

    pub fn state(&self) -> AppState {
        self.inner
            .lock()
            .expect("runtime mutex should not be poisoned")
            .state
    }

    pub fn main_window_visible(&self) -> bool {
        matches!(
            self.state(),
            AppState::VisibleIdle | AppState::VisibleTranslating | AppState::SettingsOpen
        )
    }

    pub fn busy(&self) -> bool {
        self.inner
            .lock()
            .expect("runtime mutex should not be poisoned")
            .busy
    }

    pub fn current_preset(&self) -> String {
        self.inner
            .lock()
            .expect("runtime mutex should not be poisoned")
            .current_preset
            .clone()
    }

    pub fn config(&self) -> AppConfig {
        self.inner
            .lock()
            .expect("runtime mutex should not be poisoned")
            .config
            .clone()
    }

    pub fn is_exiting(&self) -> bool {
        matches!(
            self.state(),
            AppState::Exiting | AppState::CancellingThenExit
        )
    }

    pub async fn translate_text<S, N>(
        &self,
        secrets: &S,
        notifier: &N,
        input: &str,
        language: &str,
        preset_id: &str,
    ) -> crate::error::Result<TranslationOutcome>
    where
        S: SecretStore,
        N: Notifier,
    {
        let input = input.trim();
        let language = language.trim();

        if input.is_empty() {
            return Ok(TranslationOutcome::validation_error(
                "Enter text to translate.",
            ));
        }

        if language.is_empty() {
            return Ok(TranslationOutcome::validation_error(
                "Enter target language.",
            ));
        }

        let config = self.config();
        if config.provider.model_name.trim().is_empty() {
            return Ok(TranslationOutcome::validation_error(
                "Model name is not configured. Open Settings.",
            ));
        }

        let Some(preset) = config.presets.iter().find(|preset| preset.id == preset_id) else {
            return Ok(TranslationOutcome::validation_error("Select a preset."));
        };

        let Some(api_key) = secrets.get_api_key().await? else {
            return Ok(TranslationOutcome::validation_error(
                "API key is not configured.",
            ));
        };

        if api_key.trim().is_empty() {
            return Ok(TranslationOutcome::validation_error(
                "API key is not configured.",
            ));
        }

        self.translate();
        let model = config.provider.model_name.clone();
        let provider_host = provider_host(&config.provider.base_url);
        let input_chars = input.chars().count();

        let outcome = match LlmClient::new(config.provider.clone()) {
            Ok(client) => match client.translate(&api_key, language, preset, input).await {
                Ok(translated) => {
                    tracing::info!(
                        input_chars,
                        output_chars = translated.chars().count(),
                        model = %model,
                        provider_host = %provider_host,
                        "translation completed"
                    );
                    TranslationOutcome::success(translated)
                }
                Err(error) => {
                    let kind = error.kind();
                    tracing::warn!(
                        input_chars,
                        model = %model,
                        provider_host = %provider_host,
                        error_kind = ?kind,
                        "translation failed"
                    );
                    let outcome = llm_to_outcome(error);
                    notify_failure(notifier, &outcome);
                    outcome
                }
            },
            Err(error) => {
                let kind = error.kind();
                tracing::warn!(
                    input_chars,
                    model = %model,
                    provider_host = %provider_host,
                    error_kind = ?kind,
                    "translation setup failed"
                );
                let outcome = llm_to_outcome(error);
                notify_failure(notifier, &outcome);
                outcome
            }
        };

        self.finish_translation();
        Ok(outcome)
    }

    fn finish_translation(&self) {
        let mut inner = self
            .inner
            .lock()
            .expect("runtime mutex should not be poisoned");
        inner.busy = false;
        inner.state = match inner.state {
            AppState::VisibleTranslating => AppState::VisibleIdle,
            AppState::HiddenTranslating => AppState::Hidden,
            AppState::CancellingThenExit => AppState::Exiting,
            state => state,
        };
    }
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::new("precise")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationOutcome {
    pub translated_text: Option<String>,
    pub message: Option<String>,
}

impl TranslationOutcome {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            translated_text: Some(text.into()),
            message: None,
        }
    }

    pub fn validation_error(message: impl Into<String>) -> Self {
        Self {
            translated_text: None,
            message: Some(message.into()),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            translated_text: None,
            message: Some(message.into()),
        }
    }
}

fn llm_to_outcome(error: LlmError) -> TranslationOutcome {
    TranslationOutcome::failure(provider_message(error.kind()))
}

fn provider_message(kind: LlmErrorKind) -> &'static str {
    match kind {
        LlmErrorKind::UnauthorizedOrForbidden => {
            "Provider returned 403. Check API key and model permissions."
        }
        LlmErrorKind::NotFound => "Provider returned 404. Check base URL and model name.",
        LlmErrorKind::RateLimited => "Provider returned 429. Try again later.",
        LlmErrorKind::ProviderOverloaded => "Provider returned 529. Try again later.",
        LlmErrorKind::ServerError => "Provider returned a server error. Try again later.",
        LlmErrorKind::Timeout => "Request timed out. Try again later.",
        LlmErrorKind::Network => "Network error. Check your connection and provider URL.",
        LlmErrorKind::InvalidResponse => "Provider returned an invalid response.",
        LlmErrorKind::MissingConfiguration => "Model name is not configured. Open Settings.",
    }
}

fn provider_host(base_url: &str) -> String {
    Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "invalid-provider-url".to_string())
}

fn notify_failure(notifier: &impl Notifier, outcome: &TranslationOutcome) {
    let Some(message) = outcome.message.as_deref() else {
        return;
    };

    let urgency = if message.contains("403") || message.contains("404") {
        Urgency::Critical
    } else {
        Urgency::Normal
    };
    notifier.translation_failed("Translation failed", message, urgency);
}
