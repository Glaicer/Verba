use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use verba::{
    app_runtime::{AppRuntime, TranslationOutcome},
    config::AppConfig,
    notify::{Notifier, Urgency},
    secrets::{SecretFuture, SecretStore},
};

#[tokio::test]
async fn translation_should_reject_empty_input() {
    let runtime = AppRuntime::from_config(AppConfig::default());
    let outcome = runtime
        .translate_text(
            &SecretStub::with_key("sk-test"),
            &RecordingNotifier::default(),
            "",
            "German",
            "precise",
        )
        .await
        .expect("validation outcome should be returned");

    assert_eq!(
        outcome,
        TranslationOutcome::validation_error("Enter text to translate.")
    );
}

#[tokio::test]
async fn translation_should_reject_empty_language() {
    let runtime = AppRuntime::from_config(AppConfig::default());
    let outcome = runtime
        .translate_text(
            &SecretStub::with_key("sk-test"),
            &RecordingNotifier::default(),
            "Hello",
            " ",
            "precise",
        )
        .await
        .expect("validation outcome should be returned");

    assert_eq!(
        outcome,
        TranslationOutcome::validation_error("Enter target language.")
    );
}

#[tokio::test]
async fn translation_should_prompt_settings_when_model_is_missing() {
    let runtime = AppRuntime::from_config(AppConfig::default());
    let outcome = runtime
        .translate_text(
            &SecretStub::with_key("sk-test"),
            &RecordingNotifier::default(),
            "Hello",
            "German",
            "precise",
        )
        .await
        .expect("validation outcome should be returned");

    assert_eq!(
        outcome,
        TranslationOutcome::validation_error("Model name is not configured. Open Settings.")
    );
}

#[tokio::test]
async fn translation_should_reject_missing_api_key() {
    let runtime = AppRuntime::from_config(config_with_model("http://127.0.0.1:9"));
    let outcome = runtime
        .translate_text(
            &SecretStub::missing(),
            &RecordingNotifier::default(),
            "Hello",
            "German",
            "precise",
        )
        .await
        .expect("validation outcome should be returned");

    assert_eq!(
        outcome,
        TranslationOutcome::validation_error("API key is not configured.")
    );
}

#[tokio::test]
async fn translation_should_parse_successful_response() {
    let server = OneShotServer::new(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 60\r\n\r\n{\"choices\":[{\"message\":{\"content\":\"Hallo\"}}],\"ignored\":true}",
    );
    let runtime = AppRuntime::from_config(config_with_model(&server.base_url()));

    let outcome = runtime
        .translate_text(
            &SecretStub::with_key("sk-test"),
            &RecordingNotifier::default(),
            "Hello",
            "German",
            "precise",
        )
        .await
        .expect("translation should complete");

    assert_eq!(outcome, TranslationOutcome::success("Hallo"));
}

#[tokio::test]
async fn translation_should_notify_critical_for_forbidden_provider_response() {
    let server = OneShotServer::new(
        "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\nforbidden",
    );
    let runtime = AppRuntime::from_config(config_with_model(&server.base_url()));
    let notifier = RecordingNotifier::default();

    let outcome = runtime
        .translate_text(
            &SecretStub::with_key("sk-test"),
            &notifier,
            "Hello",
            "German",
            "precise",
        )
        .await
        .expect("provider failure should become user outcome");

    assert_eq!(
        outcome,
        TranslationOutcome::failure("Provider returned 403. Check API key and model permissions.")
    );
    assert_eq!(notifier.last_urgency(), Some(Urgency::Critical));
}

#[tokio::test]
async fn translation_should_notify_critical_for_missing_provider_response() {
    let server = OneShotServer::new(
        "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 7\r\n\r\nmissing",
    );
    let runtime = AppRuntime::from_config(config_with_model(&server.base_url()));
    let notifier = RecordingNotifier::default();

    let outcome = runtime
        .translate_text(
            &SecretStub::with_key("sk-test"),
            &notifier,
            "Hello",
            "German",
            "precise",
        )
        .await
        .expect("provider failure should become user outcome");

    assert_eq!(
        outcome,
        TranslationOutcome::failure("Provider returned 404. Check base URL and model name.")
    );
    assert_eq!(notifier.last_urgency(), Some(Urgency::Critical));
}

#[tokio::test]
async fn translation_should_notify_normal_for_provider_overload() {
    let server = OneShotServer::new(
        "HTTP/1.1 529 Site Overloaded\r\nContent-Type: text/plain\r\nContent-Length: 10\r\n\r\noverloaded",
    );
    let runtime = AppRuntime::from_config(config_with_model(&server.base_url()));
    let notifier = RecordingNotifier::default();

    let outcome = runtime
        .translate_text(
            &SecretStub::with_key("sk-test"),
            &notifier,
            "Hello",
            "German",
            "precise",
        )
        .await
        .expect("provider failure should become user outcome");

    assert_eq!(
        outcome,
        TranslationOutcome::failure("Provider returned 529. Try again later.")
    );
    assert_eq!(notifier.last_urgency(), Some(Urgency::Normal));
}

fn config_with_model(base_url: &str) -> AppConfig {
    let mut config = AppConfig::default();
    config.provider.base_url = base_url.to_string();
    config.provider.model_name = "test-model".to_string();
    config
}

#[derive(Clone)]
struct SecretStub {
    value: Option<String>,
}

impl SecretStub {
    fn with_key(value: &str) -> Self {
        Self {
            value: Some(value.to_string()),
        }
    }

    fn missing() -> Self {
        Self { value: None }
    }
}

impl SecretStore for SecretStub {
    fn get_api_key(&self) -> SecretFuture<'_, Option<String>> {
        Box::pin(async move { Ok(self.value.clone()) })
    }

    fn set_api_key<'a>(&'a self, _value: &'a str) -> SecretFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }

    fn clear_api_key(&self) -> SecretFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(Default)]
struct RecordingNotifier {
    calls: Arc<Mutex<Vec<(String, String, Urgency)>>>,
}

impl RecordingNotifier {
    fn last_urgency(&self) -> Option<Urgency> {
        self.calls
            .lock()
            .expect("notifier test mutex should not be poisoned")
            .last()
            .map(|(_, _, urgency)| *urgency)
    }
}

impl Notifier for RecordingNotifier {
    fn translation_failed(&self, title: &str, message: &str, urgency: Urgency) {
        self.calls
            .lock()
            .expect("notifier test mutex should not be poisoned")
            .push((title.to_string(), message.to_string(), urgency));
    }
}

struct OneShotServer {
    base_url: String,
    handle: Option<thread::JoinHandle<String>>,
}

impl OneShotServer {
    fn new(response: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let addr = listener
            .local_addr()
            .expect("test server address should exist");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test request should connect");
            stream
                .set_read_timeout(Some(Duration::from_millis(500)))
                .expect("read timeout should be set");
            let mut request = Vec::new();
            let mut buf = [0_u8; 4096];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        request.extend_from_slice(&buf[..n]);
                        if request.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            stream
                .write_all(response.as_bytes())
                .expect("test response should write");
            String::from_utf8_lossy(&request).to_string()
        });

        Self {
            base_url: format!("http://{addr}"),
            handle: Some(handle),
        }
    }

    fn base_url(&self) -> String {
        self.base_url.clone()
    }
}

impl Drop for OneShotServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
