import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { accountList } from "../stores/accountCommands";
import {
  messageCreate,
  messageError,
  messageList,
  streamChat,
  toMessageStreamChunkEvent,
  toMessageStreamCompleteEvent,
  toMessageStreamErrorEvent
} from "../stores/chatCommands";
import type { AccountDto } from "../types/accounts";
import type { ChatDisplayState, MessageDto } from "../types/chat";
import type { PaneDefinition } from "../types/layout";

const openAiProviderId = "openai";
const defaultModelId = "OpenAIGpt";

interface PaneChatState {
  accounts: AccountDto[];
  messages: MessageDto[];
  selectedAccountId: string;
  selectedModelId: string;
  inputValue: string;
  displayState: ChatDisplayState;
  isLoading: boolean;
  error: string | null;
  canSend: boolean;
  setSelectedAccountId: (accountId: string) => void;
  setSelectedModelId: (modelId: string) => void;
  setInputValue: (value: string) => void;
  sendMessage: () => Promise<void>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Chat command failed.";
}

function activeOpenAiApiKeyAccounts(accounts: AccountDto[]): AccountDto[] {
  return accounts.filter(
    (account) =>
      account.providerId === openAiProviderId &&
      account.authType === "api_key" &&
      account.status === "active"
  );
}

function replaceMessage(messages: MessageDto[], nextMessage: MessageDto): MessageDto[] {
  const index = messages.findIndex((message) => message.id === nextMessage.id);

  if (index === -1) {
    return [...messages, nextMessage];
  }

  const nextMessages = [...messages];
  nextMessages[index] = nextMessage;
  return nextMessages;
}

function streamMessage(messages: MessageDto[], messageId: string, delta: string): MessageDto[] {
  return messages.map((message) =>
    message.id === messageId
      ? {
          ...message,
          content: `${message.content}${delta}`,
          status: "streaming"
        }
      : message
  );
}

export function usePaneChat(pane: PaneDefinition): PaneChatState {
  const [accounts, setAccounts] = useState<AccountDto[]>([]);
  const [messages, setMessages] = useState<MessageDto[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState("");
  const [selectedModelId, setSelectedModelId] = useState(pane.modelId ?? defaultModelId);
  const [inputValue, setInputValue] = useState("");
  const [displayState, setDisplayState] = useState<ChatDisplayState>("idle");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const openAiAccounts = useMemo(() => activeOpenAiApiKeyAccounts(accounts), [accounts]);

  const reloadMessages = useCallback(async () => {
    try {
      setMessages(await messageList(pane.id));
    } catch (loadError) {
      setError(errorMessage(loadError));
      setDisplayState("error");
    }
  }, [pane.id]);

  const canSend =
    inputValue.trim().length > 0 &&
    selectedAccountId.length > 0 &&
    (displayState === "idle" || displayState === "error");

  useEffect(() => {
    let isActive = true;

    async function loadChat() {
      setIsLoading(true);
      setError(null);

      try {
        const [loadedAccounts, loadedMessages] = await Promise.all([
          accountList(openAiProviderId),
          messageList(pane.id)
        ]);

        if (!isActive) {
          return;
        }

        const apiKeyAccounts = activeOpenAiApiKeyAccounts(loadedAccounts);
        const paneAccount = apiKeyAccounts.find((account) => account.id === pane.accountId);
        const defaultAccount = apiKeyAccounts.find((account) => account.isDefault);

        setAccounts(loadedAccounts);
        setMessages(loadedMessages);
        setSelectedAccountId((currentAccountId) => {
          if (apiKeyAccounts.some((account) => account.id === currentAccountId)) {
            return currentAccountId;
          }

          return paneAccount?.id ?? defaultAccount?.id ?? apiKeyAccounts[0]?.id ?? "";
        });
        setSelectedModelId(pane.modelId ?? defaultModelId);
        setDisplayState("idle");
      } catch (loadError) {
        if (isActive) {
          setError(errorMessage(loadError));
          setDisplayState("error");
        }
      } finally {
        if (isActive) {
          setIsLoading(false);
        }
      }
    }

    void loadChat();

    return () => {
      isActive = false;
    };
  }, [pane.accountId, pane.id, pane.modelId]);

  useEffect(() => {
    let isActive = true;
    const cleanupFns: Array<() => void> = [];

    async function bindStreamEvents() {
      try {
        cleanupFns.push(
          await listen("message_stream_chunk", (event) => {
            const payload = toMessageStreamChunkEvent(event.payload);

            if (!payload || !isActive || payload.paneId !== pane.id) {
              return;
            }

            setDisplayState("streaming");
            setMessages((currentMessages) =>
              streamMessage(currentMessages, payload.messageId, payload.delta)
            );
          })
        );

        cleanupFns.push(
          await listen("message_stream_complete", (event) => {
            const payload = toMessageStreamCompleteEvent(event.payload);

            if (!payload || !isActive || payload.paneId !== pane.id) {
              return;
            }

            setDisplayState("idle");
            void reloadMessages();
          })
        );

        cleanupFns.push(
          await listen("message_stream_error", (event) => {
            const payload = toMessageStreamErrorEvent(event.payload);

            if (!payload || !isActive || payload.paneId !== pane.id) {
              return;
            }

            setDisplayState("error");
            setError(payload.message ?? payload.errorCode ?? "Streaming response failed.");
            void reloadMessages();
          })
        );
      } catch (bindError) {
        setError(errorMessage(bindError));
        setDisplayState("error");
      }
    }

    void bindStreamEvents();

    return () => {
      isActive = false;
      cleanupFns.forEach((cleanup) => cleanup());
    };
  }, [pane.id, reloadMessages]);

  const sendMessage = useCallback(async () => {
    const content = inputValue.trim();

    if (content.length === 0) {
      return;
    }

    if (!selectedAccountId) {
      setError("Connect an active OpenAI API-key account before sending.");
      setDisplayState("error");
      return;
    }

    setDisplayState("sending");
    setError(null);
    setInputValue("");

    let assistantMessageId: string | null = null;

    try {
      const metadataJson = JSON.stringify({
        providerId: openAiProviderId,
        accountId: selectedAccountId,
        modelId: selectedModelId
      });
      const created = await messageCreate({
        paneId: pane.id,
        content,
        contentType: "text",
        metadataJson
      });

      assistantMessageId = created.assistantMessage.id;
      setMessages((currentMessages) =>
        replaceMessage(
          replaceMessage(currentMessages, created.userMessage),
          created.assistantMessage
        )
      );
      setDisplayState("streaming");

      await streamChat({
        paneId: pane.id,
        providerId: openAiProviderId,
        accountId: selectedAccountId,
        modelId: selectedModelId,
        assistantMessageId
      });

      await reloadMessages();
      setDisplayState("idle");
    } catch (sendError) {
      const message = errorMessage(sendError);

      if (assistantMessageId) {
        try {
          const erroredMessage = await messageError(
            assistantMessageId,
            "provider_execution_unavailable",
            message
          );
          setMessages((currentMessages) => replaceMessage(currentMessages, erroredMessage));
        } catch {
          await reloadMessages();
        }
      }

      setError(message);
      setDisplayState("error");
    }
  }, [inputValue, pane.id, reloadMessages, selectedAccountId, selectedModelId]);

  return {
    accounts: openAiAccounts,
    messages,
    selectedAccountId,
    selectedModelId,
    inputValue,
    displayState,
    isLoading,
    error,
    canSend,
    setSelectedAccountId,
    setSelectedModelId,
    setInputValue,
    sendMessage
  };
}
