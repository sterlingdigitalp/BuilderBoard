use serde::{Deserialize, Serialize};

use crate::models::{Conversation, Message, Model};
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderResolutionError {
    pub code: String,
    pub provider_id: Option<String>,
    pub provider_type: Option<String>,
    pub message: String,
}

impl ProviderResolutionError {
    pub fn provider_not_configured() -> Self {
        Self {
            code: "provider_not_configured".to_string(),
            provider_id: None,
            provider_type: None,
            message: "pane has no provider configured".to_string(),
        }
    }

    pub fn unsupported_provider(provider_id: impl Into<String>, provider_type: impl Into<String>) -> Self {
        let provider_id = provider_id.into();
        let provider_type = provider_type.into();
        Self {
            code: "unsupported_provider".to_string(),
            provider_id: Some(provider_id.clone()),
            provider_type: Some(provider_type.clone()),
            message: format!("provider '{provider_id}' with type '{provider_type}' is not supported in Phase 2C"),
        }
    }

    pub fn storage(message: String) -> Self {
        Self {
            code: "storage_error".to_string(),
            provider_id: None,
            provider_type: None,
            message,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AnthropicProvider;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OpenAIProvider;

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
    fn send(&self, _request: ProviderRequest) -> ProviderResult<ProviderResponse> {
        Err(not_implemented(Provider::OpenAI))
    }

    fn stream(&self, _request: ProviderRequest) -> ProviderResult<ProviderStream> {
        Err(not_implemented(Provider::OpenAI))
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
        "openai" => Ok(Box::new(OpenAIProvider)),
        "google" => Ok(Box::new(GoogleProvider)),
        _ => Err(ProviderResolutionError::unsupported_provider(
            provider.id.clone(),
            provider.provider_type.clone(),
        )),
    }
}

fn not_implemented(provider: Provider) -> ProviderError {
    ProviderError::NotImplemented { provider }
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_provider_for_registry_entry, AnthropicProvider, GoogleProvider, LLMProvider,
        OpenAIProvider, ProviderError,
    };
    use crate::models::{Conversation, Model};
    use crate::providers::ProviderRequest;
    use crate::storage::models::ProviderDto;

    #[test]
    fn provider_stubs_list_models_without_network() {
        assert_eq!(AnthropicProvider.list_models(), Ok(vec![Model::AnthropicClaude]));
        assert_eq!(OpenAIProvider.list_models(), Ok(vec![Model::OpenAIGpt]));
        assert_eq!(GoogleProvider.list_models(), Ok(vec![Model::GoogleGemini]));
    }

    #[test]
    fn provider_stubs_do_not_call_network() {
        let request = ProviderRequest::new(Conversation::new("conversation-1", Model::OpenAIGpt));
        let result = OpenAIProvider.send(request);

        assert!(matches!(result, Err(ProviderError::NotImplemented { .. })));
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
}
