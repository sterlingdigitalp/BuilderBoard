import { invoke } from "@tauri-apps/api/core";
import type {
  MessageCreateInput,
  MessageCreateResult,
  MessageDto,
  MessageRole,
  MessageStatus,
  MessageStreamChunkEvent,
  MessageStreamCompleteEvent,
  MessageStreamEnrichmentStartedEvent,
  MessageStreamErrorEvent,
  StreamChatInput
} from "../types/chat";

const messageRoles: MessageRole[] = ["system", "user", "assistant", "tool"];
const messageStatuses: MessageStatus[] = ["pending", "streaming", "complete", "error"];
const localMessageKey = "builderboard.localMessages.v1";

type LocalMessageState = Record<string, MessageDto[]>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function nullableString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function messageRole(value: unknown): MessageRole {
  return typeof value === "string" && messageRoles.includes(value as MessageRole)
    ? (value as MessageRole)
    : "assistant";
}

function messageStatus(value: unknown): MessageStatus {
  return typeof value === "string" && messageStatuses.includes(value as MessageStatus)
    ? (value as MessageStatus)
    : "error";
}

function nowIso(): string {
  return new Date().toISOString();
}

function readLocalMessages(): LocalMessageState {
  try {
    const rawState = window.localStorage.getItem(localMessageKey);
    if (!rawState) {
      return {};
    }

    const parsed = JSON.parse(rawState);
    return isRecord(parsed) ? (parsed as LocalMessageState) : {};
  } catch {
    return {};
  }
}

function writeLocalMessages(state: LocalMessageState): void {
  window.localStorage.setItem(localMessageKey, JSON.stringify(state));
}

function localMessagesForPane(paneId: string): MessageDto[] {
  return readLocalMessages()[paneId] ?? [];
}

function createLocalMessage(
  paneId: string,
  role: MessageRole,
  content: string,
  status: MessageStatus,
  parentId: string | null
): MessageDto {
  const now = nowIso();

  return {
    id: `local-message-${crypto.randomUUID()}`,
    workspaceId: "local",
    paneId,
    parentId,
    role,
    content,
    contentType: "text",
    status,
    providerId: role === "assistant" ? "openai" : null,
    accountId: null,
    modelId: null,
    metadataJson: "{}",
    createdAt: now,
    updatedAt: now
  };
}

export function toMessageDto(value: unknown): MessageDto {
  if (!isRecord(value) || typeof value.id !== "string" || typeof value.paneId !== "string") {
    throw new Error("Invalid message response from persistence layer.");
  }

  return {
    id: value.id,
    workspaceId: typeof value.workspaceId === "string" ? value.workspaceId : "default",
    paneId: value.paneId,
    parentId: nullableString(value.parentId),
    role: messageRole(value.role),
    content: typeof value.content === "string" ? value.content : "",
    contentType: typeof value.contentType === "string" ? value.contentType : "text",
    status: messageStatus(value.status),
    providerId: nullableString(value.providerId),
    accountId: nullableString(value.accountId),
    modelId: nullableString(value.modelId),
    metadataJson: typeof value.metadataJson === "string" ? value.metadataJson : "{}",
    createdAt: typeof value.createdAt === "string" ? value.createdAt : "",
    updatedAt: typeof value.updatedAt === "string" ? value.updatedAt : ""
  };
}

function toMessageCreateResult(value: unknown): MessageCreateResult {
  if (!isRecord(value)) {
    throw new Error("Invalid message_create response from persistence layer.");
  }

  return {
    userMessage: toMessageDto(value.userMessage),
    assistantMessage: toMessageDto(value.assistantMessage)
  };
}

export function toMessageStreamChunkEvent(value: unknown): MessageStreamChunkEvent | null {
  if (
    !isRecord(value) ||
    typeof value.paneId !== "string" ||
    typeof value.messageId !== "string" ||
    typeof value.delta !== "string"
  ) {
    return null;
  }

  return {
    paneId: value.paneId,
    messageId: value.messageId,
    delta: value.delta
  };
}

export function toMessageStreamEnrichmentStartedEvent(
  value: unknown
): MessageStreamEnrichmentStartedEvent | null {
  if (!isRecord(value) || typeof value.paneId !== "string" || typeof value.messageId !== "string") {
    return null;
  }

  return {
    paneId: value.paneId,
    messageId: value.messageId
  };
}

export function toMessageStreamCompleteEvent(value: unknown): MessageStreamCompleteEvent | null {
  if (!isRecord(value) || typeof value.paneId !== "string" || typeof value.messageId !== "string") {
    return null;
  }

  return {
    paneId: value.paneId,
    messageId: value.messageId
  };
}

export function toMessageStreamErrorEvent(value: unknown): MessageStreamErrorEvent | null {
  if (!isRecord(value) || typeof value.paneId !== "string" || typeof value.messageId !== "string") {
    return null;
  }

  return {
    paneId: value.paneId,
    messageId: value.messageId,
    errorCode: typeof value.errorCode === "string" ? value.errorCode : undefined,
    message: typeof value.message === "string" ? value.message : undefined
  };
}

export async function messageList(paneId: string): Promise<MessageDto[]> {
  try {
    const response = await invoke<unknown[]>("message_list", { paneId });
    return response.map(toMessageDto);
  } catch {
    return localMessagesForPane(paneId);
  }
}

export async function messageCreate(input: MessageCreateInput): Promise<MessageCreateResult> {
  try {
    const response = await invoke<unknown>("message_create", {
      paneId: input.paneId,
      content: input.content,
      contentType: input.contentType ?? "text",
      metadataJson: input.metadataJson ?? "{}"
    });
    return toMessageCreateResult(response);
  } catch {
    const state = readLocalMessages();
    const messages = state[input.paneId] ?? [];
    const userMessage = createLocalMessage(input.paneId, "user", input.content, "complete", null);
    const assistantMessage = createLocalMessage(input.paneId, "assistant", "", "pending", userMessage.id);

    writeLocalMessages({
      ...state,
      [input.paneId]: [...messages, userMessage, assistantMessage]
    });
    return { userMessage, assistantMessage };
  }
}

export async function messageStreamUpdate(messageId: string, delta: string): Promise<MessageDto> {
  const response = await invoke<unknown>("message_stream_update", { messageId, delta });
  return toMessageDto(response);
}

export async function messageComplete(messageId: string): Promise<MessageDto> {
  const response = await invoke<unknown>("message_complete", {
    messageId,
    content: null,
    tokenCountInput: null,
    tokenCountOutput: null,
    metadataJson: null
  });
  return toMessageDto(response);
}

export async function messageError(
  messageId: string,
  errorCode: string,
  errorMessage: string
): Promise<MessageDto> {
  try {
    const response = await invoke<unknown>("message_error", {
      messageId,
      errorCode,
      errorMessage
    });
    return toMessageDto(response);
  } catch {
    const state = readLocalMessages();
    let updatedMessage: MessageDto | null = null;
    const nextState = Object.fromEntries(
      Object.entries(state).map(([paneId, messages]) => [
        paneId,
        messages.map((message) => {
          if (message.id !== messageId) {
            return message;
          }

          updatedMessage = {
            ...message,
            status: "error",
            content: errorMessage,
            metadataJson: JSON.stringify({ errorCode }),
            updatedAt: nowIso()
          };
          return updatedMessage;
        })
      ])
    );

    writeLocalMessages(nextState);

    if (!updatedMessage) {
      throw new Error(errorMessage);
    }

    return updatedMessage;
  }
}

// Provider execution is intentionally delegated to the backend. Phase 4A UI consumes this
// command when present and degrades to a persisted assistant error when it is not registered.
export async function streamChat(input: StreamChatInput): Promise<void> {
  await invoke("stream_chat", {
    paneId: input.paneId,
    providerId: input.providerId,
    accountId: input.accountId,
    modelId: input.modelId,
    assistantMessageId: input.assistantMessageId
  });
}
