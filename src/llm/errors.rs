#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LlmErrorKind {
    UnauthorizedOrForbidden,
    NotFound,
    RateLimited,
    ProviderOverloaded,
    ServerError,
    Timeout,
    Network,
    InvalidResponse,
    MissingConfiguration,
}

#[derive(Debug, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct LlmError {
    kind: LlmErrorKind,
    message: String,
}

impl LlmError {
    pub fn new(kind: LlmErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: truncate_provider_body(message.into()),
        }
    }

    pub fn from_status(status: u16, body: &str) -> Self {
        let kind = match status {
            401 | 403 => LlmErrorKind::UnauthorizedOrForbidden,
            404 => LlmErrorKind::NotFound,
            429 => LlmErrorKind::RateLimited,
            529 => LlmErrorKind::ProviderOverloaded,
            500..=599 => LlmErrorKind::ServerError,
            _ => LlmErrorKind::InvalidResponse,
        };

        let message = if body.trim().is_empty() {
            format!("provider returned HTTP {status}")
        } else {
            format!(
                "provider returned HTTP {status}: {}",
                truncate_provider_body(body)
            )
        };

        Self { kind, message }
    }

    pub fn kind(&self) -> LlmErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

fn truncate_provider_body(body: impl AsRef<str>) -> String {
    const MAX_CHARS: usize = 512;
    let body = body.as_ref().trim();
    let mut truncated: String = body.chars().take(MAX_CHARS).collect();
    if body.chars().count() > MAX_CHARS {
        truncated.push_str("...");
    }
    truncated
}
