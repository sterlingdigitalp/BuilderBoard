use std::io::{BufRead, BufReader};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::CredentialHandle;
use crate::models::{Conversation, Message, MessageRole, Model};
use crate::storage::models::ProviderDto;

pub type ProviderResult<T> = Result<T, ProviderError>;

pub trait LLMProvider {
    fn send(&self, request: ProviderRequest) -> ProviderResult<ProviderResponse>;

    fn stream(&self, request: ProviderRequest) -> ProviderResult<ProviderStream>;

    fn list_models(&self) -> ProviderResult<Vec<Model>>;
}

pub type ProviderStream = Box<dyn Iterator<Item = ProviderResult<StreamChunk>>>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Provider {
    Anthropic,
    OpenAI,
    Google,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderRequest {
    pub conversation: Conversation,
}

impl ProviderRequest {
    pub fn new(conversation: Conversation) -> Self {
        Self { conversation }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderResponse {
    pub message: Message,
    pub model: Model,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamChunk {
    pub content_delta: String,
    pub is_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderError {
    NotImplemented { provider: Provider },
    MissingCredentials { provider: Provider },
    UnsupportedAuth { provider: Provider, auth_type: String },
    Http { status: Option<u16>, message: String },
    InvalidResponse { message: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderResolutionError {
    pub code: String,
    pub provider_id: Option<String>,
    pub provider_type: Option<String>,
    pub account_id: Option<String>,
    pub message: String,
}

impl ProviderResolutionError {
    pub fn provider_not_configured() -> Self {
        Self {
            code: "provider_not_configured".to_string(),
            provider_id: None,
            provider_type: None,
            account_id: None,
            message: "pane has no provider configured".to_string(),
        }
    }

    pub fn no_account(provider_id: impl Into<String>, account_id: Option<String>) -> Self {
        let provider_id = provider_id.into();
        let message = match account_id.as_deref() {
            Some(account_id) => format!("account '{account_id}' was not found for provider '{provider_id}'"),
            None => format!("provider '{provider_id}' has no active account"),
        };

        Self {
            code: "no_account".to_string(),
            provider_id: Some(provider_id),
            provider_type: None,
            account_id,
            message,
        }
    }

    pub fn inactive_account(
        provider_id: impl Into<String>,
        account_id: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        let provider_id = provider_id.into();
        let account_id = account_id.into();
        let status = status.into();
        Self {
            code: "inactive_account".to_string(),
            provider_id: Some(provider_id.clone()),
            provider_type: None,
            account_id: Some(account_id.clone()),
            message: format!("account '{account_id}' for provider '{provider_id}' is not active: {status}"),
        }
    }

    pub fn expired_account(provider_id: impl Into<String>, account_id: impl Into<String>) -> Self {
        let provider_id = provider_id.into();
        let account_id = account_id.into();
        Self {
            code: "expired_account".to_string(),
            provider_id: Some(provider_id.clone()),
            provider_type: None,
            account_id: Some(account_id.clone()),
            message: format!("account '{account_id}' for provider '{provider_id}' is expired"),
        }
    }

    pub fn unsupported_provider(provider_id: impl Into<String>, provider_type: impl Into<String>) -> Self {
        let provider_id = provider_id.into();
        let provider_type = provider_type.into();
        Self {
            code: "unsupported_provider".to_string(),
            provider_id: Some(provider_id.clone()),
            provider_type: Some(provider_type.clone()),
            account_id: None,
            message: format!("provider '{provider_id}' with type '{provider_type}' is not supported in Phase 3A"),
        }
    }

    pub fn storage(message: String) -> Self {
        Self {
            code: "storage_error".to_string(),
            provider_id: None,
            provider_type: None,
            account_id: None,
            message,
        }
    }
}

pub struct ResolvedProvider {
    pub provider: Box<dyn LLMProvider>,
    pub credential: CredentialHandle,
}

impl ResolvedProvider {
    pub fn new(provider: Box<dyn LLMProvider>, credential: CredentialHandle) -> Self {
        Self { provider, credential }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AnthropicProvider;

#[derive(Clone, Debug)]
pub struct OpenAIProvider {
    api_key: Option<String>,
    base_url: String,
    client: Client,
}

impl Default for OpenAIProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for OpenAIProvider {
    fn eq(&self, other: &Self) -> bool {
        self.api_key == other.api_key && self.base_url == other.base_url
    }
}

impl Eq for OpenAIProvider {}

impl OpenAIProvider {
    pub fn new() -> Self {
        Self {
            api_key: None,
            base_url: "https://api.openai.com/v1".to_string(),
            client: Client::new(),
        }
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
            base_url: "https://api.openai.com/v1".to_string(),
            client: Client::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_base_url_for_test(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
            base_url: base_url.into(),
            client: Client::new(),
        }
    }

    fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    fn api_key(&self) -> ProviderResult<&str> {
        self.api_key
            .as_deref()
            .filter(|api_key| !api_key.trim().is_empty())
            .ok_or(ProviderError::MissingCredentials {
                provider: Provider::OpenAI,
            })
    }

    fn request_body(request: ProviderRequest, stream: bool) -> serde_json::Value {
        json!({
            "model": openai_model_name(&request.conversation.model),
            "messages": openai_messages(&request.conversation),
            "stream": stream,
        })
    }

    fn send_request(&self, request: ProviderRequest, stream: bool) -> ProviderResult<reqwest::blocking::Response> {
        let api_key = self.api_key()?;
        let response = self
            .client
            .post(self.chat_completions_url())
            .bearer_auth(api_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&Self::request_body(request, stream))
            .send()
            .map_err(|error| ProviderError::Http {
                status: error.status().map(|status| status.as_u16()),
                message: error.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().unwrap_or_else(|error| error.to_string());
            return Err(ProviderError::Http {
                status: Some(status),
                message,
            });
        }

        Ok(response)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GoogleProvider;

impl LLMProvider for AnthropicProvider {
    fn send(&self, _request: ProviderRequest) -> ProviderResult<ProviderResponse> {
        Err(not_implemented(Provider::Anthropic))
    }

    fn stream(&self, _request: ProviderRequest) -> ProviderResult<ProviderStream> {
        Err(not_implemented(Provider::Anthropic))
    }

    fn list_models(&self) -> ProviderResult<Vec<Model>> {
        Ok(vec![Model::AnthropicClaude])
    }
}

impl LLMProvider for OpenAIProvider {
    fn send(&self, request: ProviderRequest) -> ProviderResult<ProviderResponse> {
        let model = request.conversation.model.clone();
        let response = self.send_request(request, false)?;
        let body: OpenAIChatCompletionResponse = response.json().map_err(|error| {
            ProviderError::InvalidResponse {
                message: error.to_string(),
            }
        })?;
        let content = body
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .ok_or_else(|| ProviderError::InvalidResponse {
                message: "OpenAI response did not include assistant content".to_string(),
            })?;

        Ok(ProviderResponse {
            message: Message::new(MessageRole::Assistant, content),
            model,
        })
    }

    fn stream(&self, request: ProviderRequest) -> ProviderResult<ProviderStream> {
        let response = self.send_request(request, true)?;
        Ok(Box::new(OpenAIStream::new(response)))
    }

    fn list_models(&self) -> ProviderResult<Vec<Model>> {
        Ok(vec![Model::OpenAIGpt])
    }
}

impl LLMProvider for GoogleProvider {
    fn send(&self, _request: ProviderRequest) -> ProviderResult<ProviderResponse> {
        Err(not_implemented(Provider::Google))
    }

    fn stream(&self, _request: ProviderRequest) -> ProviderResult<ProviderStream> {
        Err(not_implemented(Provider::Google))
    }

    fn list_models(&self) -> ProviderResult<Vec<Model>> {
        Ok(vec![Model::GoogleGemini])
    }
}

pub fn resolve_provider_for_registry_entry(
    provider: &ProviderDto,
) -> Result<Box<dyn LLMProvider>, ProviderResolutionError> {
    match provider.provider_type.as_str() {
        "anthropic" => Ok(Box::new(AnthropicProvider)),
        "openai" => Ok(Box::new(OpenAIProvider::new())),
        "google" => Ok(Box::new(GoogleProvider)),
        _ => Err(ProviderResolutionError::unsupported_provider(
            provider.id.clone(),
            provider.provider_type.clone(),
        )),
    }
}

pub fn resolve_provider_with_credential(
    provider: &ProviderDto,
    credential: CredentialHandle,
) -> Result<ResolvedProvider, ProviderResolutionError> {
    let provider = resolve_provider_for_registry_entry(provider)?;
    Ok(ResolvedProvider::new(provider, credential))
}

pub fn resolve_openai_provider_with_api_key(
    provider: &ProviderDto,
    credential: &CredentialHandle,
    api_key: String,
) -> Result<ResolvedProvider, ProviderResolutionError> {
    if provider.provider_type != "openai" {
        return Err(ProviderResolutionError::unsupported_provider(
            provider.id.clone(),
            provider.provider_type.clone(),
        ));
    }

    if credential.auth_type != "api_key" {
        return Err(ProviderResolutionError::storage(format!(
            "OpenAI execution requires api_key auth, got {}",
            credential.auth_type
        )));
    }

    Ok(ResolvedProvider::new(
        Box::new(OpenAIProvider::with_api_key(api_key)),
        credential.clone(),
    ))
}

#[derive(Debug, Deserialize)]
struct OpenAIChatCompletionResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    content: Option<String>,
}

struct OpenAIStream {
    lines: std::io::Lines<BufReader<reqwest::blocking::Response>>,
    done: bool,
}

impl OpenAIStream {
    fn new(response: reqwest::blocking::Response) -> Self {
        Self {
            lines: BufReader::new(response).lines(),
            done: false,
        }
    }
}

impl Iterator for OpenAIStream {
    type Item = ProviderResult<StreamChunk>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        for line in self.lines.by_ref() {
            let line = match line {
                Ok(line) => line,
                Err(error) => {
                    self.done = true;
                    return Some(Err(ProviderError::Http {
                        status: None,
                        message: error.to_string(),
                    }));
                }
            };
            let line = line.trim();
            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            let Some(data) = line.strip_prefix("data: ") else {
                continue;
            };
            if data == "[DONE]" {
                self.done = true;
                return Some(Ok(StreamChunk {
                    content_delta: String::new(),
                    is_complete: true,
                }));
            }

            let event = match serde_json::from_str::<OpenAIStreamResponse>(data) {
                Ok(event) => event,
                Err(error) => {
                    self.done = true;
                    return Some(Err(ProviderError::InvalidResponse {
                        message: error.to_string(),
                    }));
                }
            };

            if let Some(choice) = event.choices.into_iter().next() {
                if choice.finish_reason.is_some() {
                    self.done = true;
                    return Some(Ok(StreamChunk {
                        content_delta: String::new(),
                        is_complete: true,
                    }));
                }
                if let Some(content_delta) = choice.delta.content {
                    return Some(Ok(StreamChunk {
                        content_delta,
                        is_complete: false,
                    }));
                }
            }
        }

        self.done = true;
        None
    }
}

fn openai_model_name(model: &Model) -> String {
    match model {
        Model::OpenAIGpt => "gpt-4o-mini".to_string(),
        Model::Custom(model) => model.clone(),
        _ => "gpt-4o-mini".to_string(),
    }
}

fn openai_messages(conversation: &Conversation) -> Vec<serde_json::Value> {
    conversation
        .messages
        .iter()
        .map(|message| {
            json!({
                "role": openai_role(&message.role),
                "content": message.content,
            })
        })
        .collect()
}

fn openai_role(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    }
}

fn not_implemented(provider: Provider) -> ProviderError {
    ProviderError::NotImplemented { provider }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    use super::{
        resolve_provider_for_registry_entry, AnthropicProvider, GoogleProvider, LLMProvider,
        OpenAIProvider, ProviderError,
    };
    use crate::models::{Conversation, Message, MessageRole, Model};
    use crate::providers::ProviderRequest;
    use crate::storage::models::ProviderDto;

    #[test]
    fn provider_stubs_list_models_without_network() {
        assert_eq!(AnthropicProvider.list_models(), Ok(vec![Model::AnthropicClaude]));
        assert_eq!(OpenAIProvider::new().list_models(), Ok(vec![Model::OpenAIGpt]));
        assert_eq!(GoogleProvider.list_models(), Ok(vec![Model::GoogleGemini]));
    }

    #[test]
    fn provider_stubs_do_not_call_network() {
        let request = ProviderRequest::new(Conversation::new("conversation-1", Model::OpenAIGpt));
        let result = AnthropicProvider.send(request);

        assert!(matches!(result, Err(ProviderError::NotImplemented { .. })));
    }

    #[test]
    fn openai_send_posts_chat_completion_json() {
        let (base_url, request_rx) = spawn_openai_server(
            "HTTP/1.1 200 OK",
            "application/json",
            r#"{"choices":[{"message":{"content":"Hello from OpenAI"}}]}"#,
        );
        let provider = OpenAIProvider::with_base_url_for_test("sk-test", base_url);

        let response = provider
            .send(ProviderRequest::new(openai_hello_conversation()))
            .expect("OpenAI send should parse response");

        assert_eq!(response.message.content, "Hello from OpenAI");
        let request = request_rx.recv().expect("server should capture request");
        let lower_request = request.to_ascii_lowercase();
        assert!(request.starts_with("POST /chat/completions HTTP/1.1"));
        assert!(lower_request.contains("content-type: application/json"));
        assert!(lower_request.contains("authorization: bearer sk-test"));
        assert!(request.contains(r#""model":"gpt-4o-mini""#));
        assert!(request.contains(r#""role":"user""#));
        assert!(request.contains(r#""content":"Hello""#));
        assert!(request.contains(r#""stream":false"#));
    }

    #[test]
    fn openai_stream_parses_sse_chunks() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":null}]}\n\n",
            "data: [DONE]\n\n",
        );
        let (base_url, request_rx) = spawn_openai_server("HTTP/1.1 200 OK", "text/event-stream", body);
        let provider = OpenAIProvider::with_base_url_for_test("sk-test", base_url);

        let chunks = provider
            .stream(ProviderRequest::new(openai_hello_conversation()))
            .expect("OpenAI stream should start")
            .collect::<Result<Vec<_>, _>>()
            .expect("OpenAI stream should parse chunks");

        assert_eq!(chunks[0].content_delta, "Hel");
        assert_eq!(chunks[1].content_delta, "lo");
        assert!(chunks[2].is_complete);
        let request = request_rx.recv().expect("server should capture request");
        assert!(request.contains(r#""stream":true"#));
    }

    #[test]
    fn openai_invalid_key_returns_http_error() {
        let (base_url, _request_rx) = spawn_openai_server(
            "HTTP/1.1 401 Unauthorized",
            "application/json",
            r#"{"error":{"message":"Invalid API key"}}"#,
        );
        let provider = OpenAIProvider::with_base_url_for_test("sk-invalid", base_url);

        let error = provider
            .send(ProviderRequest::new(openai_hello_conversation()))
            .expect_err("invalid key should return provider error");

        assert!(matches!(error, ProviderError::Http { status: Some(401), .. }));
    }

    #[test]
    fn openai_without_api_key_returns_missing_credentials() {
        let error = OpenAIProvider::new()
            .send(ProviderRequest::new(openai_hello_conversation()))
            .expect_err("unbound OpenAI provider should not execute");

        assert!(matches!(error, ProviderError::MissingCredentials { .. }));
    }

    #[test]
    fn mvp_provider_types_resolve() {
        assert_eq!(resolve("anthropic").list_models().unwrap(), vec![Model::AnthropicClaude]);
        assert_eq!(resolve("openai").list_models().unwrap(), vec![Model::OpenAIGpt]);
        assert_eq!(resolve("google").list_models().unwrap(), vec![Model::GoogleGemini]);
    }

    #[test]
    fn unsupported_provider_type_returns_structured_error() {
        let provider = provider_dto("openrouter");
        let error = match resolve_provider_for_registry_entry(&provider) {
            Ok(_) => panic!("openrouter is not supported in Phase 2C"),
            Err(error) => error,
        };

        assert_eq!(error.code, "unsupported_provider");
        assert_eq!(error.provider_id.as_deref(), Some("openrouter"));
        assert_eq!(error.provider_type.as_deref(), Some("openrouter"));
    }

    fn resolve(provider_type: &str) -> Box<dyn LLMProvider> {
        let provider = provider_dto(provider_type);
        resolve_provider_for_registry_entry(&provider).expect("provider should resolve")
    }

    fn provider_dto(provider_type: &str) -> ProviderDto {
        ProviderDto {
            id: provider_type.to_string(),
            provider_type: provider_type.to_string(),
            display_name: provider_type.to_string(),
            enabled: true,
            auth_mode: "api_key".to_string(),
            supports_chat: true,
            supports_streaming: true,
            supports_tool_use: false,
            supports_vision: false,
            context_window: None,
            locality: "remote".to_string(),
        }
    }

    fn openai_hello_conversation() -> Conversation {
        Conversation::new("conversation-1", Model::OpenAIGpt)
            .with_message(Message::new(MessageRole::User, "Hello"))
    }

    fn spawn_openai_server(
        status_line: &'static str,
        content_type: &'static str,
        body: &'static str,
    ) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let (request_tx, request_rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buffer = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let read = stream.read(&mut chunk).expect("read request");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            let headers = String::from_utf8_lossy(&buffer).to_string();
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            let header_end = buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|position| position + 4)
                .unwrap_or(buffer.len());
            let mut body_bytes_read = buffer.len().saturating_sub(header_end);
            while body_bytes_read < content_length {
                let read = stream.read(&mut chunk).expect("read request body");
                if read == 0 {
                    break;
                }
                body_bytes_read += read;
                buffer.extend_from_slice(&chunk[..read]);
            }

            request_tx
                .send(String::from_utf8_lossy(&buffer).to_string())
                .expect("send captured request");

            let response = format!(
                "{status_line}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        (format!("http://{address}"), request_rx)
    }
}
