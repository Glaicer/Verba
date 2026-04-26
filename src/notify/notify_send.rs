use std::process::Command;

use crate::notify::{Notifier, Urgency};

#[derive(Clone, Debug, Default)]
pub struct NotifySend;

impl NotifySend {
    pub fn args(title: &str, message: &str, urgency: Urgency) -> Vec<String> {
        vec![
            "--app-name=Verba".to_string(),
            format!("--urgency={}", urgency.as_str()),
            title.to_string(),
            message.to_string(),
        ]
    }
}

impl Notifier for NotifySend {
    fn translation_failed(&self, title: &str, message: &str, urgency: Urgency) {
        let args = Self::args(title, message, urgency);
        std::thread::spawn(move || {
            let _ = Command::new("notify-send").args(args).status();
        });
    }
}

impl Urgency {
    fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Critical => "critical",
        }
    }
}
