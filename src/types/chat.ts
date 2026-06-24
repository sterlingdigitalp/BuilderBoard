export type MessageRole = "system" | "user" | "assistant" | "tool";
export type MessageStatus = "pending" | "streaming" | "complete" | "error";
export type ChatDisplayState = "idle" | "sending" | "enriching" | "streaming" | "error";

export interface MessageDto {
  id: string;
  workspaceId: string;
  paneId: string;
  parentId: string | null;
  role: MessageRole;
  content: string;
  contentType: string;
  status: MessageStatus;
  providerId: string | null;
  accountId: string | null;
  modelId: string | null;
  metadataJson: string;
  createdAt: string;
  updatedAt: string;
}

export interface MessageCreateResult {
  userMessage: MessageDto;
  assistantMessage: MessageDto;
}

export interface MessageCreateInput {
  paneId: string;
  content: string;
  contentType?: string;
  metadataJson?: string;
}

export interface StreamChatInput {
  paneId: string;
  providerId: "openai";
  accountId: string;
  modelId: string;
  assistantMessageId: string;
}

export interface MessageStreamChunkEvent {
  paneId: string;
  messageId: string;
  delta: string;
}

export interface MessageStreamCompleteEvent {
  paneId: string;
  messageId: string;
}

export interface MessageStreamErrorEvent {
  paneId: string;
  messageId: string;
  errorCode?: string;
  message?: string;
}

export interface MessageStreamEnrichmentStartedEvent {
  paneId: string;
  messageId: string;
}
