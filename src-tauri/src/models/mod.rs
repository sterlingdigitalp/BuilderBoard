use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Model {
    AnthropicClaude,
    OpenAIGpt,
    GoogleGemini,
    Custom(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

impl Message {
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

pub fn message_role_from_db(role: &str) -> Option<MessageRole> {
    match role {
        "system" => Some(MessageRole::System),
        "user" => Some(MessageRole::User),
        "assistant" => Some(MessageRole::Assistant),
        "tool" => Some(MessageRole::Tool),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub model: Model,
    pub messages: Vec<Message>,
}

impl Conversation {
    pub fn new(id: impl Into<String>, model: Model) -> Self {
        Self {
            id: id.into(),
            model,
            messages: Vec::new(),
        }
    }

    pub fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{Conversation, Message, MessageRole, Model};

    #[test]
    fn conversation_collects_messages() {
        let conversation = Conversation::new("conversation-1", Model::OpenAIGpt)
            .with_message(Message::new(MessageRole::User, "Hello"));

        assert_eq!(conversation.messages.len(), 1);
        assert_eq!(conversation.messages[0].content, "Hello");
    }

    #[test]
    fn message_role_from_db_includes_tool() {
        assert_eq!(
            super::message_role_from_db("tool"),
            Some(MessageRole::Tool)
        );
    }
}
