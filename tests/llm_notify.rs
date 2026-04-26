use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

use verba::{
    config::{AppConfig, Preset},
    llm::{
        client::LlmClient,
        errors::{LlmError, LlmErrorKind},
        prompt::build_system_prompt,
    },
    notify::{notify_send::NotifySend, Urgency},
};

#[test]
fn prompt_should_use_strict_translation_format() {
    let preset = Preset {
        id: "precise".to_string(),
        name: "Precise".to_string(),
        instruction: "Be exact.".to_string(),
    };

    let prompt = build_system_prompt("German", &preset);

    assert!(prompt.contains("Translate this text into German."));
    assert!(prompt.contains("Style and quality requirements:\nBe exact."));
    assert!(prompt.contains("- Return only the translated text."));
    assert!(prompt.contains("- Preserve markdown structure."));
}

#[test]
fn notify_send_should_build_required_arguments() {
    let args = NotifySend::args(
        "Translation failed",
        "Provider returned 403. Check API key and model permissions.",
        Urgency::Critical,
    );

    assert_eq!(
        args,
        [
            "--app-name=Verba",
            "--urgency=critical",
            "Translation failed",
            "Provider returned 403. Check API key and model permissions."
        ]
    );
}

#[test]
fn llm_error_mapping_should_cover_required_status_codes() {
    assert_eq!(
        LlmError::from_status(403, "forbidden").kind(),
        LlmErrorKind::UnauthorizedOrForbidden
    );
    assert_eq!(
        LlmError::from_status(404, "missing").kind(),
        LlmErrorKind::NotFound
    );
    assert_eq!(
        LlmError::from_status(429, "slow down").kind(),
        LlmErrorKind::RateLimited
    );
    assert_eq!(
        LlmError::from_status(529, "overloaded").kind(),
        LlmErrorKind::ProviderOverloaded
    );
    assert_eq!(
        LlmError::from_status(500, "server").kind(),
        LlmErrorKind::ServerError
    );
}

#[tokio::test]
async fn llm_client_should_parse_successful_chat_completion_response() {
    let server = OneShotServer::new(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 65\r\n\r\n{\"choices\":[{\"message\":{\"content\":\"Hallo Welt\"}}],\"ignored\":true}",
    );
    let mut config = AppConfig::default();
    config.provider.base_url = server.base_url();
    config.provider.model_name = "test-model".to_string();
    let client = LlmClient::new(config.provider.clone()).expect("client should build");

    let translated = client
        .translate("secret", "German", &config.presets[0], "Hello world")
        .await
        .expect("translation should parse");

    assert_eq!(translated, "Hallo Welt");
    assert!(server
        .request()
        .contains("POST /v1/chat/completions HTTP/1.1"));
}

#[tokio::test]
async fn llm_client_should_reject_missing_message_content() {
    let server = OneShotServer::new(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 28\r\n\r\n{\"choices\":[{\"message\":{}}]}",
    );
    let mut config = AppConfig::default();
    config.provider.base_url = server.base_url();
    config.provider.model_name = "test-model".to_string();
    let client = LlmClient::new(config.provider.clone()).expect("client should build");

    let err = client
        .translate("secret", "German", &config.presets[0], "Hello world")
        .await
        .expect_err("missing content should fail");

    assert_eq!(err.kind(), LlmErrorKind::InvalidResponse);
}

struct OneShotServer {
    base_url: String,
    handle: thread::JoinHandle<String>,
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
            handle,
        }
    }

    fn base_url(&self) -> String {
        self.base_url.clone()
    }

    fn request(self) -> String {
        self.handle.join().expect("test server should finish")
    }
}
