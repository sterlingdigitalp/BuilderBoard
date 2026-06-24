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
  toMessageStreamEnrichmentStartedEvent,
  toMessageStreamErrorEvent
} from "../stores/chatCommands";
import { paneSettingsFor, updatePaneSettings } from "../stores/paneSettingsStore";
import { probeCrossPaneInteraction, runtimeTraceEnabled } from "../stores/runtimeDiagnostics";
import type { AccountDto } from "../types/accounts";
import type { ChatDisplayState, MessageDto } from "../types/chat";
import type { PaneDefinition } from "../types/layout";
import type { OpenAiModelId, ReasoningLevel } from "../types/paneSettings";

const openAiProviderId = "openai";

export interface PaneChatState {
  accounts: AccountDto[];
  messages: MessageDto[];
  selectedAccountId: string;
  selectedModelId: OpenAiModelId;
  selectedReasoningLevel: ReasoningLevel;
  inputValue: string;
  displayState: ChatDisplayState;
  isLoading: boolean;
  error: string | null;
  canSend: boolean;
  setSelectedAccountId: (accountId: string) => void;
  selectModel: (modelId: OpenAiModelId) => void;
  selectReasoning: (reasoningLevel: ReasoningLevel) => void;
  setInputValue: (value: string) => void;
  sendMessage: () => Promise<void>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Chat command failed.";
}

function activeOpenAiAccounts(accounts: AccountDto[]): AccountDto[] {
  return accounts.filter((account) => account.providerId === openAiProviderId && account.status === "active");
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
  const initialSettings = paneSettingsFor(pane);
  const [accounts, setAccounts] = useState<AccountDto[]>([]);
  const [messages, setMessages] = useState<MessageDto[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState("");
  const [selectedModelId, setSelectedModelId] = useState(initialSettings.modelId);
  const [selectedReasoningLevel, setSelectedReasoningLevel] =
    useState(initialSettings.reasoningLevel);
  const [inputValue, setInputValue] = useState("");
  const [displayState, setDisplayState] = useState<ChatDisplayState>("idle");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const openAiAccounts = useMemo(() => activeOpenAiAccounts(accounts), [accounts]);

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

        const openAiAccounts = activeOpenAiAccounts(loadedAccounts);
        const paneAccount = openAiAccounts.find((account) => account.id === pane.accountId);
        const defaultAccount = openAiAccounts.find((account) => account.isDefault);
        const settings = paneSettingsFor(pane);

        setAccounts(loadedAccounts);
        setMessages(loadedMessages);
        setSelectedAccountId((currentAccountId) => {
          if (openAiAccounts.some((account) => account.id === currentAccountId)) {
            return currentAccountId;
          }

          return paneAccount?.id ?? defaultAccount?.id ?? openAiAccounts[0]?.id ?? "";
        });
        setSelectedModelId(settings.modelId);
        setSelectedReasoningLevel(settings.reasoningLevel);
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

    return () => { isActive = false; };
  }, [pane.accountId, pane.id, pane.modelId]);

  const selectModel = useCallback((modelId: OpenAiModelId) => {
    setSelectedModelId(updatePaneSettings(pane, { modelId }).modelId);
  }, [pane]);

  const selectReasoning = useCallback((reasoningLevel: ReasoningLevel) => {
    setSelectedReasoningLevel(updatePaneSettings(pane, { reasoningLevel }).reasoningLevel);
  }, [pane]);

  useEffect(() => {
    let isActive = true;
    const cleanupFns: Array<() => void> = [];
    const pendingDeltas = new Map<string, string>();
    let flushFrameId: number | null = null;

    const flushPendingDeltas = () => {
      flushFrameId = null;
      if (!isActive || pendingDeltas.size === 0) {
        return;
      }

      const deltas = new Map(pendingDeltas);
      pendingDeltas.clear();
      setDisplayState("streaming");
      setMessages((currentMessages) => {
        let nextMessages = currentMessages;
        for (const [messageId, delta] of deltas) {
          nextMessages = streamMessage(nextMessages, messageId, delta);
        }
        return nextMessages;
      });
    };

    const queueStreamDelta = (messageId: string, delta: string) => {
      pendingDeltas.set(messageId, `${pendingDeltas.get(messageId) ?? ""}${delta}`);
      if (flushFrameId !== null) {
        return;
      }
      flushFrameId = window.requestAnimationFrame(flushPendingDeltas);
    };

    async function bindStreamEvents() {
      try {
        cleanupFns.push(
          await listen("message_stream_enrichment_started", (event) => {
            const payload = toMessageStreamEnrichmentStartedEvent(event.payload);

            if (!payload || !isActive || payload.paneId !== pane.id) {
              return;
            }

            setDisplayState("enriching");
          })
        );

        cleanupFns.push(
          await listen("message_stream_chunk", (event) => {
            const payload = toMessageStreamChunkEvent(event.payload);

            if (!payload || !isActive || payload.paneId !== pane.id) {
              return;
            }

            queueStreamDelta(payload.messageId, payload.delta);
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
      if (flushFrameId !== null) {
        window.cancelAnimationFrame(flushFrameId);
      }
      pendingDeltas.clear();
      cleanupFns.forEach((cleanup) => cleanup());
    };
  }, [pane.id, reloadMessages]);

  useEffect(() => {
    if (!runtimeTraceEnabled()) {
      return;
    }

    if (displayState !== "enriching" && displayState !== "streaming") {
      return;
    }

    const intervalId = window.setInterval(() => {
      void probeCrossPaneInteraction(pane.id).catch(() => {});
    }, 1500);

    void probeCrossPaneInteraction(pane.id).catch(() => {});

    return () => {
      window.clearInterval(intervalId);
    };
  }, [displayState, pane.id]);

  const sendMessage = useCallback(async () => {
    const content = inputValue.trim();

    if (content.length === 0) {
      return;
    }

    if (!selectedAccountId) {
      setError("Connect an active OpenAI account before sending.");
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
        modelId: selectedModelId,
        reasoningLevel: selectedReasoningLevel
      });
      const created = await messageCreate({
        paneId: pane.id,
        content,
        contentType: "text",
        metadataJson
      });

      assistantMessageId = created.assistantMessage.id;
      setMessages((currentMessages) =>
        replaceMessage(replaceMessage(currentMessages, created.userMessage), created.assistantMessage)
      );
      setDisplayState("streaming");

      void streamChat({
        paneId: pane.id,
        providerId: openAiProviderId,
        accountId: selectedAccountId,
        modelId: selectedModelId,
        assistantMessageId
      }).catch(async (streamError) => {
        const message = errorMessage(streamError);

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
      });
    } catch (sendError) {
      const message = errorMessage(sendError);

      if (assistantMessageId) {
        try {
          const erroredMessage = await messageError(assistantMessageId, "provider_execution_unavailable", message);
          setMessages((currentMessages) => replaceMessage(currentMessages, erroredMessage));
        } catch {
          await reloadMessages();
        }
      }

      setError(message);
      setDisplayState("error");
    }
  }, [inputValue, pane.id, reloadMessages, selectedAccountId, selectedModelId, selectedReasoningLevel]);

  return {
    accounts: openAiAccounts,
    messages,
    selectedAccountId,
    selectedModelId,
    selectedReasoningLevel,
    inputValue,
    displayState,
    isLoading,
    error,
    canSend,
    setSelectedAccountId,
    selectModel,
    selectReasoning,
    setInputValue,
    sendMessage
  };
}
