import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { accountList } from "../stores/accountCommands";
import { engineList, type EngineInfo } from "../stores/engineCommands";
import { builderList, type BuilderInfo } from "../stores/builderCommands";
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
import type { EffortLevel, ModelId } from "../types/paneSettings";

export interface PaneChatState {
  accounts: AccountDto[];
  builders: BuilderInfo[];
  selectedBuilderId: string;
  engines: EngineInfo[];
  messages: MessageDto[];
  selectedEngineId: string;
  selectedAccountId: string;
  selectedModelId: ModelId;
  selectedEffort: EffortLevel;
  inputValue: string;
  displayState: ChatDisplayState;
  isLoading: boolean;
  isMessageLoading: boolean;
  error: string | null;
  builderError: string | null;
  engineError: string | null;
  accountError: string | null;
  messageError: string | null;
  canSend: boolean;
  setSelectedAccountId: (accountId: string) => void;
  selectBuilder: (builderName: string) => void;
  selectEngine: (engineId: string) => void;
  selectModel: (modelId: ModelId) => void;
  selectEffort: (effort: EffortLevel) => void;
  setInputValue: (value: string) => void;
  sendMessage: () => Promise<void>;
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  return error instanceof Error ? error.message : "Chat command failed.";
}

function activeOpenAiAccounts(accounts: AccountDto[]): AccountDto[] {
  return accounts.filter((account) => account.providerId === "openai" && account.status === "active");
}

function firstSupportedValue<T extends string>(values: T[], current: T, fallback: T): T {
  return values.includes(current) ? current : values[0] ?? fallback;
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
  const [engines, setEngines] = useState<EngineInfo[]>([]);
  const [builders, setBuilders] = useState<BuilderInfo[]>([]);
  const [selectedBuilderId, setSelectedBuilderId] = useState("");
  const [messages, setMessages] = useState<MessageDto[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState("");
  const [selectedEngineId, setSelectedEngineId] = useState(initialSettings.engineId);
  const [selectedModelId, setSelectedModelId] = useState(initialSettings.modelId);
  const [selectedEffort, setSelectedEffort] = useState<EffortLevel>(initialSettings.effort);
  // Builder selected separately, can auto-apply to engine/model/effort
  const [inputValue, setInputValue] = useState("");
  const [displayState, setDisplayState] = useState<ChatDisplayState>("idle");
  const [isLoading, setIsLoading] = useState(true);
  const [isMessageLoading, setIsMessageLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [builderError, setBuilderError] = useState<string | null>(null);
  const [engineError, setEngineError] = useState<string | null>(null);
  const [accountError, setAccountError] = useState<string | null>(null);
  const [messageLoadError, setMessageLoadError] = useState<string | null>(null);

  const reloadMessages = useCallback(async () => {
    try {
      setMessages(await messageList(pane.id));
      setMessageLoadError(null);
    } catch (loadError) {
      setMessageLoadError(`Message history unavailable: ${errorMessage(loadError)}`);
    }
  }, [pane.id]);

  const canSend =
    inputValue.trim().length > 0 &&
    (selectedEngineId === "grok" || selectedAccountId.length > 0) &&
    (displayState === "idle" || displayState === "error");

  useEffect(() => {
    let isActive = true;

    async function loadChat() {
      setIsLoading(true);
      setIsMessageLoading(true);
      setError(null);

      const [accountsResult, messagesResult, enginesResult, buildersResult] = await Promise.allSettled([
        accountList("openai"),
        messageList(pane.id),
        engineList(),
        builderList()
      ]);

      if (!isActive) {
        return;
      }

      const loadedAccounts = accountsResult.status === "fulfilled" ? accountsResult.value : [];
      const loadedMessages = messagesResult.status === "fulfilled" ? messagesResult.value : [];
      const loadedEngines = enginesResult.status === "fulfilled" ? enginesResult.value : [];
      const loadedBuilders = buildersResult.status === "fulfilled" ? buildersResult.value : [];

      setAccountError(
        accountsResult.status === "rejected" ? `Account lookup failed: ${errorMessage(accountsResult.reason)}` : null
      );
      setMessageLoadError(
        messagesResult.status === "rejected"
          ? `Message history unavailable: ${errorMessage(messagesResult.reason)}`
          : null
      );
      setEngineError(
        enginesResult.status === "rejected" ? `Engine discovery failed: ${errorMessage(enginesResult.reason)}` : null
      );
      setBuilderError(
        buildersResult.status === "rejected"
          ? `Builder registry unavailable: ${errorMessage(buildersResult.reason)}`
          : null
      );

      const settings = paneSettingsFor(pane);
      const activeEngine = loadedEngines.find((e) => e.id === settings.engineId) || loadedEngines[0];
      const activeEngineId = activeEngine?.id ?? settings.engineId;
      const activeModelId = firstSupportedValue(
        activeEngine?.models ?? [],
        settings.modelId,
        settings.modelId
      ) as ModelId;
      const activeEffort = firstSupportedValue(
        (activeEngine?.supportedEfforts ?? []) as EffortLevel[],
        settings.effort,
        settings.effort
      );

      setAccounts(loadedAccounts);
      setEngines(loadedEngines);
      setBuilders(loadedBuilders);
      setMessages(loadedMessages);
      const initialBuilder =
        loadedBuilders.find((b) => b.execution.preferredEngine === activeEngineId)?.name ??
        loadedBuilders[0]?.name ??
        "";
      setSelectedBuilderId(initialBuilder);
      setSelectedAccountId((currentAccountId) => {
        const openAiAccounts = loadedAccounts.filter((a) => a.providerId === "openai" && a.status === "active");
        if (openAiAccounts.some((account) => account.id === currentAccountId)) {
          return currentAccountId;
        }
        const paneAccount = openAiAccounts.find((account) => account.id === pane.accountId);
        const defaultAccount = openAiAccounts.find((account) => account.isDefault);
        return paneAccount?.id ?? defaultAccount?.id ?? openAiAccounts[0]?.id ?? "";
      });
      setSelectedEngineId(activeEngineId);
      setSelectedModelId(activeModelId);
      setSelectedEffort(activeEffort);
      setDisplayState("idle");
      setIsLoading(false);
      setIsMessageLoading(false);
    }

    void loadChat();

    return () => { isActive = false; };
  }, [pane.accountId, pane.id, pane.metadataJson, pane.modelId, pane.providerId]);

  const selectEngine = useCallback((engineId: string) => {
    const nextEngine = engines.find((engine) => engine.id === engineId);
    const nextModel = firstSupportedValue(nextEngine?.models ?? [], selectedModelId, selectedModelId) as ModelId;
    const nextEffort = firstSupportedValue(
      (nextEngine?.supportedEfforts ?? []) as EffortLevel[],
      selectedEffort,
      selectedEffort
    );
    const next = updatePaneSettings(pane, {
      engineId,
      modelId: nextModel,
      effort: nextEffort
    });
    setSelectedEngineId(next.engineId);
    setSelectedModelId(next.modelId);
    setSelectedEffort(next.effort);
    setSelectedBuilderId(
      builders.find((builder) => builder.execution.preferredEngine === next.engineId)?.name ?? ""
    );
  }, [builders, engines, pane, selectedEffort, selectedModelId]);

  const selectBuilder = useCallback((builderName: string) => {
    const builder = builders.find((b) => b.name === builderName);
    if (builder) {
      const exec = builder.execution;
      setSelectedBuilderId(builderName);
      const nextSettings = updatePaneSettings(pane, {
        engineId: exec.preferredEngine,
        modelId: exec.defaultModel,
        effort: (exec.effort as EffortLevel) || "medium",
      });
      setSelectedEngineId(nextSettings.engineId);
      setSelectedModelId(nextSettings.modelId);
      setSelectedEffort(nextSettings.effort);
    }
  }, [pane, builders]);

  const selectModel = useCallback((modelId: ModelId) => {
    setSelectedModelId(updatePaneSettings(pane, { modelId }).modelId);
  }, [pane]);

  const selectEffort = useCallback((effort: EffortLevel) => {
    setSelectedEffort(updatePaneSettings(pane, { effort }).effort);
  }, [pane]);

  useEffect(() => {
    if (builders.length === 0) {
      if (selectedBuilderId) {
        setSelectedBuilderId("");
      }
      return;
    }

    const selectedBuilder = builders.find((builder) => builder.name === selectedBuilderId);
    if (selectedBuilder?.execution.preferredEngine === selectedEngineId) {
      return;
    }

    setSelectedBuilderId(
      builders.find((builder) => builder.execution.preferredEngine === selectedEngineId)?.name ?? ""
    );
  }, [builders, selectedBuilderId, selectedEngineId]);

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
      setError("Select a connected account before sending.");
      setDisplayState("error");
      return;
    }

    setDisplayState("sending");
    setError(null);
    setInputValue("");

    let assistantMessageId: string | null = null;

    try {
      const metadataJson = JSON.stringify({
        providerId: selectedEngineId,
        accountId: selectedAccountId,
        modelId: selectedModelId,
        reasoningLevel: selectedEffort
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
        providerId: selectedEngineId,
        builderId: selectedBuilderId || undefined,
        accountId: selectedAccountId,
        modelId: selectedModelId,
        assistantMessageId,
        reasoningLevel: selectedEffort
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
  }, [
    inputValue,
    pane.id,
    reloadMessages,
    selectedAccountId,
    selectedBuilderId,
    selectedEffort,
    selectedEngineId,
    selectedModelId
  ]);

  return {
    accounts,
    messages,
    builders,
    selectedBuilderId,
    engines,
    selectedEngineId,
    selectedAccountId,
    selectedModelId,
    selectedEffort,
    inputValue,
    displayState,
    isLoading,
    isMessageLoading,
    error,
    builderError,
    engineError,
    accountError,
    messageError: messageLoadError,
    canSend,
    setSelectedAccountId,
    selectBuilder,
    selectEngine,
    selectModel,
    selectEffort,
    setInputValue,
    sendMessage
  };
}
