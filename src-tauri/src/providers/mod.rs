use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::sync::OnceLock;

use futures_util::StreamExt;
use reqwest::blocking::Client as BlockingClient;
use reqwest::Client as AsyncClient;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::CredentialHandle;
use crate::execution::tool_transport::{
    native_tool_name_map, NativeToolCall, NativeToolDefinition,
};
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

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderRequest {
    pub conversation: Conversation,
    pub reasoning_level: Option<String>,
    pub native_tools: Vec<NativeToolDefinition>,
    pub trace_round: Option<u32>,
}

impl ProviderRequest {
    pub fn new(conversation: Conversation) -> Self {
        Self {
            conversation,
            reasoning_level: None,
            native_tools: vec![],
            trace_round: None,
        }
    }

    pub fn with_reasoning_level(mut self, reasoning_level: Option<String>) -> Self {
        self.reasoning_level = reasoning_level;
        self
    }

    pub fn with_native_tools(mut self, native_tools: Vec<NativeToolDefinition>) -> Self {
        self.native_tools = native_tools;
        self
    }

    pub fn with_trace_round(mut self, trace_round: Option<u32>) -> Self {
        self.trace_round = trace_round;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderResponse {
    pub message: Message,
    pub model: Model,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamChunk {
    pub content_delta: String,
    pub is_complete: bool,
    pub tool_calls: Vec<NativeToolCall>,
}

impl StreamChunk {
    fn text(content_delta: impl Into<String>) -> Self {
        Self {
            content_delta: content_delta.into(),
            is_complete: false,
            tool_calls: vec![],
        }
    }

    fn complete() -> Self {
        Self {
            content_delta: String::new(),
            is_complete: true,
            tool_calls: vec![],
        }
    }

    fn tool_call(call: NativeToolCall) -> Self {
        Self {
            content_delta: String::new(),
            is_complete: false,
            tool_calls: vec![call],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderError {
    NotImplemented {
        provider: Provider,
    },
    MissingCredentials {
        provider: Provider,
    },
    UnsupportedAuth {
        provider: Provider,
        auth_type: String,
    },
    Http {
        status: Option<u16>,
        message: String,
    },
    InvalidResponse {
        message: String,
    },
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
            Some(account_id) => {
                format!("account '{account_id}' was not found for provider '{provider_id}'")
            }
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
            message: format!(
                "account '{account_id}' for provider '{provider_id}' is not active: {status}"
            ),
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

    pub fn unsupported_provider(
        provider_id: impl Into<String>,
        provider_type: impl Into<String>,
    ) -> Self {
        let provider_id = provider_id.into();
        let provider_type = provider_type.into();
        Self {
            code: "unsupported_provider".to_string(),
            provider_id: Some(provider_id.clone()),
            provider_type: Some(provider_type.clone()),
            account_id: None,
            message: format!(
                "provider '{provider_id}' with type '{provider_type}' is not supported in Phase 3A"
            ),
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
        Self {
            provider,
            credential,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AnthropicProvider;

#[derive(Debug)]
pub struct OpenAIProvider {
    auth: Option<OpenAIAuth>,
    base_url: String,
    /// Lazily initialized so async execution never creates/drops reqwest's blocking runtime
    /// on a tokio worker thread (that drop panics).
    blocking_client: OnceLock<BlockingClient>,
    async_client: AsyncClient,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum OpenAIAuth {
    ApiKey(String),
    ChatGptOAuth {
        access_token: String,
        account_id: Option<String>,
    },
}

impl Default for OpenAIProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for OpenAIProvider {
    fn eq(&self, other: &Self) -> bool {
        self.auth == other.auth && self.base_url == other.base_url
    }
}

impl Eq for OpenAIProvider {}

impl Clone for OpenAIProvider {
    fn clone(&self) -> Self {
        Self {
            auth: self.auth.clone(),
            base_url: self.base_url.clone(),
            blocking_client: OnceLock::new(),
            async_client: self.async_client.clone(),
        }
    }
}

impl OpenAIProvider {
    pub fn new() -> Self {
        Self {
            auth: None,
            base_url: "https://api.openai.com/v1".to_string(),
            blocking_client: OnceLock::new(),
            async_client: AsyncClient::new(),
        }
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            auth: Some(OpenAIAuth::ApiKey(api_key.into())),
            base_url: "https://api.openai.com/v1".to_string(),
            blocking_client: OnceLock::new(),
            async_client: AsyncClient::new(),
        }
    }

    pub fn with_bearer_token(token: impl Into<String>) -> Self {
        Self::with_api_key(token)
    }

    pub fn with_chatgpt_oauth_token(token: impl Into<String>, account_id: Option<String>) -> Self {
        Self {
            auth: Some(OpenAIAuth::ChatGptOAuth {
                access_token: token.into(),
                account_id,
            }),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            blocking_client: OnceLock::new(),
            async_client: AsyncClient::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_base_url_for_test(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            auth: Some(OpenAIAuth::ApiKey(api_key.into())),
            base_url: base_url.into(),
            blocking_client: OnceLock::new(),
            async_client: AsyncClient::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_chatgpt_base_url_for_test(
        token: impl Into<String>,
        account_id: Option<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            auth: Some(OpenAIAuth::ChatGptOAuth {
                access_token: token.into(),
                account_id,
            }),
            base_url: base_url.into(),
            blocking_client: OnceLock::new(),
            async_client: AsyncClient::new(),
        }
    }

    fn blocking_http_client(&self) -> &BlockingClient {
        self.blocking_client.get_or_init(BlockingClient::new)
    }

    fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    fn chatgpt_responses_url(&self) -> String {
        format!("{}/responses", self.base_url.trim_end_matches('/'))
    }

    fn api_key(&self) -> ProviderResult<&str> {
        match self.auth.as_ref() {
            Some(OpenAIAuth::ApiKey(api_key)) if !api_key.trim().is_empty() => Ok(api_key),
            _ => Err(ProviderError::MissingCredentials {
                provider: Provider::OpenAI,
            }),
        }
    }

    fn chatgpt_auth(&self) -> ProviderResult<(&str, Option<&str>)> {
        match self.auth.as_ref() {
            Some(OpenAIAuth::ChatGptOAuth {
                access_token,
                account_id,
            }) if !access_token.trim().is_empty() => Ok((access_token, account_id.as_deref())),
            _ => Err(ProviderError::MissingCredentials {
                provider: Provider::OpenAI,
            }),
        }
    }

    fn is_chatgpt_oauth(&self) -> bool {
        matches!(self.auth, Some(OpenAIAuth::ChatGptOAuth { .. }))
    }

    fn request_body(request: ProviderRequest, stream: bool) -> serde_json::Value {
        let native_tools = request.native_tools.clone();
        let mut body = json!({
            "model": openai_model_name(&request.conversation.model),
            "messages": openai_messages(&request.conversation),
            "stream": stream,
        });
        apply_openai_native_tools(&mut body, &native_tools);
        body
    }

    fn chatgpt_responses_body(request: ProviderRequest, stream: bool) -> serde_json::Value {
        let mut instructions = Vec::new();
        let mut input = Vec::new();
        let native_tools = request.native_tools.clone();
        for message in &request.conversation.messages {
            match message.role {
                MessageRole::System => instructions.push(message.content.clone()),
                MessageRole::User => input.push(json!({
                    "role": "user",
                    "content": [{ "type": "input_text", "text": message.content }],
                })),
                MessageRole::Assistant => input.push(json!({
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": message.content }],
                })),
                MessageRole::Tool => {
                    instructions.push(format!("Tool result:\n{}", message.content))
                }
            }
        }

        let mut body = json!({
            "model": openai_model_name(&request.conversation.model),
            "input": input,
            "store": false,
            "stream": stream,
        });
        if !instructions.is_empty() {
            body["instructions"] = json!(instructions.join("\n"));
        }
        apply_responses_native_tools(&mut body, &native_tools);
        body
    }

    fn send_request(
        &self,
        request: ProviderRequest,
        stream: bool,
    ) -> ProviderResult<reqwest::blocking::Response> {
        let model_id = openai_model_name(&request.conversation.model);
        let request_builder = match self.auth.as_ref() {
            Some(OpenAIAuth::ApiKey(_)) => {
                let api_key = match self.api_key() {
                    Ok(api_key) => api_key,
                    Err(error) => {
                        trace_openai_request_sent(false);
                        trace_openai_response_status("not_sent");
                        return Err(error);
                    }
                };
                let endpoint = self.chat_completions_url();
                trace_provider_adapter("api_key", &endpoint, &model_id);
                let trace_round = request.trace_round;
                let body = Self::request_body(request, stream);
                if let Some(round) = trace_round {
                    crate::native_tool_trace::write_request(round, &body);
                }
                self.blocking_http_client()
                    .post(endpoint)
                    .bearer_auth(api_key)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .json(&body)
            }
            Some(OpenAIAuth::ChatGptOAuth { .. }) => {
                let (access_token, account_id) = match self.chatgpt_auth() {
                    Ok(auth) => auth,
                    Err(error) => {
                        trace_openai_request_sent(false);
                        trace_openai_response_status("not_sent");
                        return Err(error);
                    }
                };
                let endpoint = self.chatgpt_responses_url();
                trace_provider_adapter("oauth", &endpoint, &model_id);
                let trace_round = request.trace_round;
                let body = Self::chatgpt_responses_body(request, stream);
                if let Some(round) = trace_round {
                    crate::native_tool_trace::write_request(round, &body);
                }
                let mut builder = self
                    .blocking_http_client()
                    .post(endpoint)
                    .bearer_auth(access_token)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .header("originator", "opencode")
                    .json(&body);
                if let Some(account_id) = account_id {
                    builder = builder.header("ChatGPT-Account-Id", account_id);
                }
                builder
            }
            None => {
                trace_openai_request_sent(false);
                trace_openai_response_status("not_sent");
                return Err(ProviderError::MissingCredentials {
                    provider: Provider::OpenAI,
                });
            }
        };

        trace_openai_request_sent(true);
        let response = request_builder.send().map_err(|error| {
            let status = error.status().map(|status| status.as_u16());
            match status {
                Some(status) => trace_openai_response_status(status),
                None => trace_openai_response_status("send_failed"),
            }
            ProviderError::Http {
                status,
                message: error.to_string(),
            }
        })?;

        trace_openai_response_status(response.status().as_u16());
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

    async fn send_request_async(
        &self,
        request: ProviderRequest,
        stream: bool,
    ) -> ProviderResult<reqwest::Response> {
        let model_id = openai_model_name(&request.conversation.model);
        let request_builder = match self.auth.as_ref() {
            Some(OpenAIAuth::ApiKey(_)) => {
                let api_key = self.api_key()?;
                let endpoint = self.chat_completions_url();
                trace_provider_adapter("api_key", &endpoint, &model_id);
                let trace_round = request.trace_round;
                let body = Self::request_body(request, stream);
                if let Some(round) = trace_round {
                    crate::native_tool_trace::write_request(round, &body);
                }
                self.async_client
                    .post(endpoint)
                    .bearer_auth(api_key)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .json(&body)
            }
            Some(OpenAIAuth::ChatGptOAuth { .. }) => {
                let (access_token, account_id) = self.chatgpt_auth()?;
                let endpoint = self.chatgpt_responses_url();
                trace_provider_adapter("oauth", &endpoint, &model_id);
                let trace_round = request.trace_round;
                let body = Self::chatgpt_responses_body(request, stream);
                if let Some(round) = trace_round {
                    crate::native_tool_trace::write_request(round, &body);
                }
                let mut builder = self
                    .async_client
                    .post(endpoint)
                    .bearer_auth(access_token)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .header("originator", "opencode")
                    .json(&body);
                if let Some(account_id) = account_id {
                    builder = builder.header("ChatGPT-Account-Id", account_id);
                }
                builder
            }
            None => {
                return Err(ProviderError::MissingCredentials {
                    provider: Provider::OpenAI,
                });
            }
        };

        trace_openai_request_sent(true);
        let response = request_builder.send().await.map_err(|error| {
            let status = error.status().map(|status| status.as_u16());
            ProviderError::Http {
                status,
                message: error.to_string(),
            }
        })?;

        trace_openai_response_status(response.status().as_u16());
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response
                .text()
                .await
                .unwrap_or_else(|error| error.to_string());
            return Err(ProviderError::Http {
                status: Some(status),
                message,
            });
        }

        Ok(response)
    }

    pub async fn stream_chunks_async<F>(
        &self,
        request: ProviderRequest,
        mut on_chunk: F,
    ) -> ProviderResult<()>
    where
        F: FnMut(ProviderResult<StreamChunk>) -> ProviderResult<()>,
    {
        let trace_round = request.trace_round;
        let native_tool_name_map = native_tool_name_map(&request.native_tools);
        let response = self.send_request_async(request, true).await?;
        let is_oauth = self.is_chatgpt_oauth();
        let mut byte_stream = response.bytes_stream();
        let mut pending = String::new();
        let mut chat_tool_calls: HashMap<u64, PartialNativeToolCall> = HashMap::new();
        let mut responses_tool_calls: HashMap<String, PartialNativeToolCall> = HashMap::new();

        while let Some(next_bytes) = byte_stream.next().await {
            let bytes = next_bytes.map_err(|error| ProviderError::Http {
                status: None,
                message: error.to_string(),
            })?;
            pending.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(line) = take_sse_line(&mut pending) {
                if let Some(round) = trace_round {
                    crate::native_tool_trace::append_provider_event(
                        round,
                        provider_stream_trace_event(line.as_str(), is_oauth),
                    );
                }
                let events = parse_sse_line(
                    line.as_str(),
                    is_oauth,
                    &native_tool_name_map,
                    &mut chat_tool_calls,
                    &mut responses_tool_calls,
                );
                for chunk in events {
                    on_chunk(chunk)?;
                }
            }
        }

        Ok(())
    }
}

fn take_sse_line(buffer: &mut String) -> Option<String> {
    let newline_index = buffer.find('\n')?;
    let mut line = buffer[..newline_index].trim().to_string();
    let rest = buffer[newline_index + 1..].to_string();
    *buffer = rest;
    if line.ends_with('\r') {
        line.pop();
    }
    if line.is_empty() {
        return take_sse_line(buffer);
    }
    Some(line)
}

#[derive(Clone, Debug, Default)]
struct PartialNativeToolCall {
    call_id: String,
    name: String,
    arguments: String,
}

fn apply_openai_native_tools(body: &mut serde_json::Value, tools: &[NativeToolDefinition]) {
    if tools.is_empty() {
        return;
    }

    body["tools"] = json!(tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }
            })
        })
        .collect::<Vec<_>>());
    body["tool_choice"] = json!("auto");
}

fn apply_responses_native_tools(body: &mut serde_json::Value, tools: &[NativeToolDefinition]) {
    if tools.is_empty() {
        return;
    }

    body["tools"] = json!(tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters,
            })
        })
        .collect::<Vec<_>>());
    body["tool_choice"] = json!("auto");
}

fn provider_stream_trace_event(line: &str, is_oauth: bool) -> serde_json::Value {
    let data = line.trim().strip_prefix("data: ").unwrap_or(line.trim());
    let parsed = serde_json::from_str::<serde_json::Value>(data).ok();
    let mut event = json!({
        "raw_line": line,
        "transport": if is_oauth { "responses" } else { "chat_completions" },
        "raw_data": data,
    });

    if let Some(parsed) = parsed {
        event["parsed"] = parsed.clone();
        if is_oauth {
            event["type"] = parsed.get("type").cloned().unwrap_or_else(|| json!(null));
            event["content_delta"] = parsed.get("delta").cloned().unwrap_or_else(|| json!(null));
            if let Some(item) = parsed.get("item") {
                event["tool_call_id"] = item
                    .get("call_id")
                    .or_else(|| item.get("id"))
                    .cloned()
                    .unwrap_or_else(|| json!(null));
                event["function_name"] = item.get("name").cloned().unwrap_or_else(|| json!(null));
                event["arguments"] = item
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!(null));
            }
        } else if let Some(choice) = parsed
            .get("choices")
            .and_then(|value| value.as_array())
            .and_then(|choices| choices.first())
        {
            event["type"] = json!("chat.completion.chunk");
            event["finish_reason"] = choice
                .get("finish_reason")
                .cloned()
                .unwrap_or_else(|| json!(null));
            if let Some(delta) = choice.get("delta") {
                event["content_delta"] =
                    delta.get("content").cloned().unwrap_or_else(|| json!(null));
                event["tool_call_delta"] = delta
                    .get("tool_calls")
                    .cloned()
                    .unwrap_or_else(|| json!(null));
            }
        }
    } else if data == "[DONE]" {
        event["type"] = json!("done");
    } else {
        event["type"] = json!("unparsed");
    }

    event
}

fn parse_sse_line(
    line: &str,
    is_oauth: bool,
    tool_name_map: &HashMap<String, String>,
    chat_tool_calls: &mut HashMap<u64, PartialNativeToolCall>,
    responses_tool_calls: &mut HashMap<String, PartialNativeToolCall>,
) -> Vec<ProviderResult<StreamChunk>> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(':') || line.starts_with("event: ") {
        return vec![];
    }
    let Some(data) = line.strip_prefix("data: ") else {
        return vec![];
    };
    if data == "[DONE]" {
        let mut chunks = if is_oauth {
            drain_responses_tool_calls(responses_tool_calls, tool_name_map)
        } else {
            drain_chat_tool_calls(chat_tool_calls, tool_name_map)
        };
        chunks.push(Ok(StreamChunk::complete()));
        return chunks;
    }

    if is_oauth {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else {
            return vec![];
        };
        match event.get("type").and_then(|value| value.as_str()) {
            Some("response.output_text.delta") => {
                let content_delta = event
                    .get("delta")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string();
                if content_delta.is_empty() {
                    vec![]
                } else {
                    vec![Ok(StreamChunk::text(content_delta))]
                }
            }
            Some("response.output_item.added") | Some("response.output_item.done") => {
                record_responses_tool_item(&event, responses_tool_calls);
                if event.get("type").and_then(|value| value.as_str())
                    == Some("response.output_item.done")
                {
                    drain_responses_tool_calls(responses_tool_calls, tool_name_map)
                } else {
                    vec![]
                }
            }
            Some("response.function_call_arguments.delta") => {
                record_responses_tool_arguments_delta(&event, responses_tool_calls);
                vec![]
            }
            Some("response.completed") => {
                let mut chunks = drain_responses_tool_calls(responses_tool_calls, tool_name_map);
                chunks.push(Ok(StreamChunk::complete()));
                chunks
            }
            Some("error") => vec![Err(ProviderError::InvalidResponse {
                message: event.to_string(),
            })],
            _ => vec![],
        }
    } else {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else {
            return vec![];
        };
        let Some(choice) = event
            .get("choices")
            .and_then(|value| value.as_array())
            .and_then(|choices| choices.first())
        else {
            return vec![];
        };
        record_chat_tool_deltas(choice, chat_tool_calls);
        if choice
            .get("finish_reason")
            .and_then(|value| value.as_str())
            .is_some()
        {
            let mut chunks = drain_chat_tool_calls(chat_tool_calls, tool_name_map);
            chunks.push(Ok(StreamChunk::complete()));
            return chunks;
        }
        let content_delta = choice
            .get("delta")
            .and_then(|delta| delta.get("content"))
            .and_then(|content| content.as_str())
            .unwrap_or_default()
            .to_string();
        if content_delta.is_empty() {
            vec![]
        } else {
            vec![Ok(StreamChunk::text(content_delta))]
        }
    }
}

fn record_chat_tool_deltas(
    choice: &serde_json::Value,
    calls: &mut HashMap<u64, PartialNativeToolCall>,
) {
    let Some(tool_calls) = choice
        .get("delta")
        .and_then(|delta| delta.get("tool_calls"))
        .and_then(|value| value.as_array())
    else {
        return;
    };

    for call in tool_calls {
        let index = call
            .get("index")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let entry = calls.entry(index).or_default();
        if let Some(id) = call.get("id").and_then(|value| value.as_str()) {
            entry.call_id = id.to_string();
        }
        if let Some(function) = call.get("function") {
            if let Some(name) = function.get("name").and_then(|value| value.as_str()) {
                entry.name.push_str(name);
            }
            if let Some(arguments) = function.get("arguments").and_then(|value| value.as_str()) {
                entry.arguments.push_str(arguments);
            }
        }
    }
}

fn record_responses_tool_item(
    event: &serde_json::Value,
    calls: &mut HashMap<String, PartialNativeToolCall>,
) {
    let Some(item) = event.get("item") else {
        return;
    };
    if item.get("type").and_then(|value| value.as_str()) != Some("function_call") {
        return;
    }

    let item_id = item
        .get("id")
        .or_else(|| item.get("call_id"))
        .and_then(|value| value.as_str())
        .unwrap_or("tool_call")
        .to_string();
    let entry = calls.entry(item_id.clone()).or_default();
    entry.call_id = item
        .get("call_id")
        .and_then(|value| value.as_str())
        .unwrap_or(&item_id)
        .to_string();
    if let Some(name) = item.get("name").and_then(|value| value.as_str()) {
        entry.name = name.to_string();
    }
    if let Some(arguments) = item.get("arguments").and_then(|value| value.as_str()) {
        entry.arguments = arguments.to_string();
    }
}

fn record_responses_tool_arguments_delta(
    event: &serde_json::Value,
    calls: &mut HashMap<String, PartialNativeToolCall>,
) {
    let Some(delta) = event.get("delta").and_then(|value| value.as_str()) else {
        return;
    };
    let item_id = event
        .get("item_id")
        .or_else(|| event.get("call_id"))
        .and_then(|value| value.as_str())
        .unwrap_or("tool_call")
        .to_string();
    calls.entry(item_id).or_default().arguments.push_str(delta);
}

fn drain_chat_tool_calls(
    calls: &mut HashMap<u64, PartialNativeToolCall>,
    tool_name_map: &HashMap<String, String>,
) -> Vec<ProviderResult<StreamChunk>> {
    let mut indexes: Vec<u64> = calls.keys().cloned().collect();
    indexes.sort_unstable();
    indexes
        .into_iter()
        .filter_map(|index| calls.remove(&index))
        .filter_map(|call| native_call_from_partial(call, tool_name_map))
        .map(|call| Ok(StreamChunk::tool_call(call)))
        .collect()
}

fn drain_responses_tool_calls(
    calls: &mut HashMap<String, PartialNativeToolCall>,
    tool_name_map: &HashMap<String, String>,
) -> Vec<ProviderResult<StreamChunk>> {
    let mut keys: Vec<String> = calls.keys().cloned().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|key| calls.remove(&key))
        .filter_map(|call| native_call_from_partial(call, tool_name_map))
        .map(|call| Ok(StreamChunk::tool_call(call)))
        .collect()
}

fn native_call_from_partial(
    call: PartialNativeToolCall,
    tool_name_map: &HashMap<String, String>,
) -> Option<NativeToolCall> {
    if call.name.trim().is_empty() {
        return None;
    }
    let tool_name = tool_name_map.get(&call.name).cloned().unwrap_or(call.name);
    let arguments = if call.arguments.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&call.arguments).unwrap_or_else(|_| json!({ "_raw": call.arguments }))
    };
    Some(NativeToolCall {
        call_id: if call.call_id.trim().is_empty() {
            format!("call_{}", tool_name.replace('.', "_"))
        } else {
            call.call_id
        },
        tool_name,
        arguments,
    })
}

fn trace_provider_adapter(auth_type: &str, endpoint: &str, model_id: &str) {
    if std::env::var("BUILDERBOARD_TRACE_OPENAI_EXECUTION").as_deref() != Ok("1") {
        return;
    }

    println!("MODEL_ID={model_id}");
    println!("ENDPOINT={endpoint}");
    println!("TRACE CredentialService -> Provider Adapter auth_type={auth_type}");
    println!("TRACE Provider Adapter -> OpenAIProvider endpoint={endpoint} model_id={model_id}");
}

fn trace_openai_request_sent(sent: bool) {
    if std::env::var("BUILDERBOARD_TRACE_OPENAI_EXECUTION").as_deref() == Ok("1") {
        crate::runtime_diagnostics::trace_runtime_metric("ENGINE_REQUEST_SENT", sent);
    }
}

fn trace_openai_response_status(status: impl std::fmt::Display) {
    if std::env::var("BUILDERBOARD_TRACE_OPENAI_EXECUTION").as_deref() == Ok("1") {
        crate::runtime_diagnostics::trace_runtime_metric("ENGINE_RESPONSE_STATUS", status);
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
        if self.is_chatgpt_oauth() {
            let response = self.send_request(request, false)?;
            let body: serde_json::Value =
                response
                    .json()
                    .map_err(|error| ProviderError::InvalidResponse {
                        message: error.to_string(),
                    })?;
            let content =
                extract_responses_text(&body).ok_or_else(|| ProviderError::InvalidResponse {
                    message: "ChatGPT response did not include assistant content".to_string(),
                })?;

            return Ok(ProviderResponse {
                message: Message::new(MessageRole::Assistant, content),
                model,
            });
        }

        let response = self.send_request(request, false)?;
        let body: OpenAIChatCompletionResponse =
            response
                .json()
                .map_err(|error| ProviderError::InvalidResponse {
                    message: error.to_string(),
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
        if self.is_chatgpt_oauth() {
            Ok(Box::new(OpenAIResponsesStream::new(response)))
        } else {
            Ok(Box::new(OpenAIStream::new(response)))
        }
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

pub fn resolve_openai_provider_with_bearer_token(
    provider: &ProviderDto,
    credential: &CredentialHandle,
    token: String,
    account_id: Option<String>,
) -> Result<ResolvedProvider, ProviderResolutionError> {
    if provider.provider_type != "openai" {
        return Err(ProviderResolutionError::unsupported_provider(
            provider.id.clone(),
            provider.provider_type.clone(),
        ));
    }

    if credential.auth_type != "api_key" && credential.auth_type != "oauth" {
        return Err(ProviderResolutionError::storage(format!(
            "OpenAI execution requires api_key or oauth auth, got {}",
            credential.auth_type
        )));
    }

    Ok(ResolvedProvider::new(
        Box::new(OpenAIProvider::with_chatgpt_oauth_token(token, account_id)),
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

struct OpenAIResponsesStream {
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

impl OpenAIResponsesStream {
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
                return Some(Ok(StreamChunk::complete()));
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
                    return Some(Ok(StreamChunk::complete()));
                }
                if let Some(content_delta) = choice.delta.content {
                    return Some(Ok(StreamChunk::text(content_delta)));
                }
            }
        }

        self.done = true;
        None
    }
}

impl Iterator for OpenAIResponsesStream {
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
            if line.is_empty() || line.starts_with(':') || line.starts_with("event: ") {
                continue;
            }
            let Some(data) = line.strip_prefix("data: ") else {
                continue;
            };
            if data == "[DONE]" {
                self.done = true;
                return Some(Ok(StreamChunk::complete()));
            }

            let event = match serde_json::from_str::<serde_json::Value>(data) {
                Ok(event) => event,
                Err(error) => {
                    self.done = true;
                    return Some(Err(ProviderError::InvalidResponse {
                        message: error.to_string(),
                    }));
                }
            };

            match event.get("type").and_then(|value| value.as_str()) {
                Some("response.output_text.delta") => {
                    let content_delta = event
                        .get("delta")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string();
                    if !content_delta.is_empty() {
                        return Some(Ok(StreamChunk::text(content_delta)));
                    }
                }
                Some("response.completed") => {
                    self.done = true;
                    return Some(Ok(StreamChunk::complete()));
                }
                Some("error") => {
                    self.done = true;
                    return Some(Err(ProviderError::InvalidResponse {
                        message: event.to_string(),
                    }));
                }
                _ => {}
            }
        }

        self.done = true;
        None
    }
}

fn extract_responses_text(body: &serde_json::Value) -> Option<String> {
    let mut pieces = Vec::new();
    for item in body.get("output")?.as_array()? {
        for content in item.get("content").and_then(|value| value.as_array())? {
            if let Some(text) = content.get("text").and_then(|value| value.as_str()) {
                pieces.push(text.to_string());
            }
        }
    }

    if pieces.is_empty() {
        None
    } else {
        Some(pieces.join(""))
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
        MessageRole::Tool => "tool",
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
    use crate::execution::tool_transport::NativeToolDefinition;
    use crate::models::{Conversation, Message, MessageRole, Model};
    use crate::providers::ProviderRequest;
    use crate::storage::models::ProviderDto;

    #[test]
    fn provider_stubs_list_models_without_network() {
        assert_eq!(
            AnthropicProvider.list_models(),
            Ok(vec![Model::AnthropicClaude])
        );
        assert_eq!(
            OpenAIProvider::new().list_models(),
            Ok(vec![Model::OpenAIGpt])
        );
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
    fn openai_send_uses_selected_custom_model() {
        for model_id in ["gpt-5.5", "gpt-5.4-mini", "gpt-5.3-codex-spark"] {
            let (base_url, request_rx) = spawn_openai_server(
                "HTTP/1.1 200 OK",
                "application/json",
                r#"{"choices":[{"message":{"content":"Hello from selected model"}}]}"#,
            );
            let provider = OpenAIProvider::with_base_url_for_test("sk-test", base_url);
            let conversation =
                Conversation::new("conversation-1", Model::Custom(model_id.to_string()))
                    .with_message(Message::new(MessageRole::User, "Hello"));

            provider
                .send(ProviderRequest::new(conversation))
                .expect("OpenAI send should parse selected model response");

            let request = request_rx.recv().expect("server should capture request");
            assert!(request.contains(&format!(r#""model":"{model_id}""#)));
        }
    }

    #[test]
    fn openai_stream_parses_sse_chunks() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":null}]}\n\n",
            "data: [DONE]\n\n",
        );
        let (base_url, request_rx) =
            spawn_openai_server("HTTP/1.1 200 OK", "text/event-stream", body);
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
    fn openai_request_includes_native_tool_definitions() {
        let (base_url, request_rx) = spawn_openai_server(
            "HTTP/1.1 200 OK",
            "application/json",
            r#"{"choices":[{"message":{"content":"Ready"}}]}"#,
        );
        let provider = OpenAIProvider::with_base_url_for_test("sk-test", base_url);
        let native_tools = vec![NativeToolDefinition {
            name: "filesystem_write".to_string(),
            tool_id: "filesystem.write".to_string(),
            description: "Write a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
        }];

        provider
            .send(ProviderRequest::new(openai_hello_conversation()).with_native_tools(native_tools))
            .expect("OpenAI send should include native tool definitions");

        let request = request_rx.recv().expect("server should capture request");
        assert!(request.contains(r#""tools":["#));
        assert!(request.contains(r#""type":"function""#));
        assert!(request.contains(r#""name":"filesystem_write""#));
        assert!(request.contains(r#""tool_choice":"auto""#));
        assert!(!request.contains("tool_call"));
    }

    #[test]
    fn openai_async_stream_emits_native_tool_call_chunk() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call-1\",\"type\":\"function\",\"function\":{\"name\":\"filesystem_write\",\"arguments\":\"{\\\"path\\\":\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"tmp/native.txt\\\",\\\"content\\\":\\\"hello\\\"}\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
            "data: [DONE]\n\n",
        );
        let (base_url, _request_rx) =
            spawn_openai_server("HTTP/1.1 200 OK", "text/event-stream", body);
        let provider = OpenAIProvider::with_base_url_for_test("sk-test", base_url);
        let native_tools = vec![NativeToolDefinition {
            name: "filesystem_write".to_string(),
            tool_id: "filesystem.write".to_string(),
            description: "Write a file".to_string(),
            parameters: serde_json::json!({ "type": "object", "properties": {}, "required": [] }),
        }];
        let mut chunks = Vec::new();

        tauri::async_runtime::block_on(provider.stream_chunks_async(
            ProviderRequest::new(openai_hello_conversation()).with_native_tools(native_tools),
            |chunk| {
                chunks.push(chunk?);
                Ok(())
            },
        ))
        .expect("OpenAI async stream should parse native tool call");

        let tool_call = chunks
            .iter()
            .flat_map(|chunk| chunk.tool_calls.iter())
            .next()
            .expect("native tool call should be emitted");
        assert_eq!(tool_call.call_id, "call-1");
        assert_eq!(tool_call.tool_name, "filesystem.write");
        assert_eq!(tool_call.arguments["path"], "tmp/native.txt");
        assert_eq!(tool_call.arguments["content"], "hello");
        assert!(chunks.iter().any(|chunk| chunk.is_complete));
    }

    #[test]
    fn openai_chatgpt_oauth_posts_responses_request() {
        let body = concat!(
            "event: response.output_text.delta\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hel\"}\n\n",
            "event: response.output_text.delta\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"lo\"}\n\n",
            "event: response.completed\n",
            "data: {\"type\":\"response.completed\",\"response\":{}}\n\n",
        );
        let (base_url, request_rx) =
            spawn_openai_server("HTTP/1.1 200 OK", "text/event-stream", body);
        let provider = OpenAIProvider::with_chatgpt_base_url_for_test(
            "oauth-access-token",
            Some("acc-openai".to_string()),
            base_url,
        );

        let chunks = provider
            .stream(ProviderRequest::new(openai_hello_conversation()))
            .expect("ChatGPT OAuth stream should start")
            .collect::<Result<Vec<_>, _>>()
            .expect("ChatGPT OAuth stream should parse chunks");

        assert_eq!(chunks[0].content_delta, "Hel");
        assert_eq!(chunks[1].content_delta, "lo");
        assert!(chunks[2].is_complete);

        let request = request_rx.recv().expect("server should capture request");
        let lower_request = request.to_ascii_lowercase();
        assert!(request.starts_with("POST /responses HTTP/1.1"));
        assert!(lower_request.contains("authorization: bearer oauth-access-token"));
        assert!(lower_request.contains("chatgpt-account-id: acc-openai"));
        assert!(request.contains("originator: opencode"));
        assert!(request.contains(r#""text":"Hello""#));
        assert!(request.contains(r#""type":"input_text""#));
        assert!(request.contains(r#""role":"user""#));
        assert!(request.contains(r#""store":false"#));
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

        assert!(matches!(
            error,
            ProviderError::Http {
                status: Some(401),
                ..
            }
        ));
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
        assert_eq!(
            resolve("anthropic").list_models().unwrap(),
            vec![Model::AnthropicClaude]
        );
        assert_eq!(
            resolve("openai").list_models().unwrap(),
            vec![Model::OpenAIGpt]
        );
        assert_eq!(
            resolve("google").list_models().unwrap(),
            vec![Model::GoogleGemini]
        );
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

    #[test]
    fn openai_provider_async_only_drop_does_not_panic_on_tokio_worker() {
        tauri::async_runtime::block_on(async {
            let provider = OpenAIProvider::with_chatgpt_oauth_token("oauth-token", None);
            drop(provider);
        });
    }
}
