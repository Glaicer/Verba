#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuiAction {
    Translate,
    OpenSettings,
    Close,
    FocusLanguage,
    FocusPreset,
    CopyResult,
}

impl GuiAction {
    pub fn action_name(self) -> &'static str {
        match self {
            Self::Translate => "translate",
            Self::OpenSettings => "settings",
            Self::Close => "close",
            Self::FocusLanguage => "focus-language",
            Self::FocusPreset => "focus-preset",
            Self::CopyResult => "copy-result",
        }
    }

    pub fn detailed_action_name(self) -> String {
        format!("win.{}", self.action_name())
    }
}

pub fn accelerators_for_action(action: GuiAction) -> &'static [&'static str] {
    match action {
        GuiAction::Translate => &["<Control>Return"],
        GuiAction::Close => &["Escape"],
        GuiAction::FocusLanguage => &["<Control>L"],
        GuiAction::FocusPreset => &["<Control>P"],
        GuiAction::CopyResult => &["<Control><Shift>C"],
        GuiAction::OpenSettings => &[],
    }
}
