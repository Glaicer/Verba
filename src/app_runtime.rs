use std::sync::{Arc, Mutex};

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
}

#[derive(Clone, Debug)]
pub struct AppRuntime {
    inner: Arc<Mutex<RuntimeState>>,
}

impl AppRuntime {
    pub fn new(current_preset: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeState {
                state: AppState::Hidden,
                busy: false,
                current_preset: current_preset.into(),
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

    pub fn is_exiting(&self) -> bool {
        matches!(
            self.state(),
            AppState::Exiting | AppState::CancellingThenExit
        )
    }
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::new("precise")
    }
}
