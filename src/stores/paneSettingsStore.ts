import type { PaneDefinition } from "../types/layout";
import type {
  ModelOption,
  OpenAiModelId,
  PaneSettings,
  ReasoningLevel,
  ReasoningOption
} from "../types/paneSettings";

const paneSettingsKey = "builderboard.paneSettings.v1";
const defaultModelId: OpenAiModelId = "gpt-5.5";
const defaultReasoningLevel: ReasoningLevel = "medium";

export const openAiModelOptions: ModelOption[] = [
  { id: "gpt-5.5", label: "GPT-5.5" },
  { id: "gpt-5.4-mini", label: "GPT-5.4 mini" },
  { id: "gpt-5.3-codex-spark", label: "GPT-5.3 Codex Spark" }
];

export const reasoningOptions: ReasoningOption[] = [
  { id: "low", label: "Low" },
  { id: "medium", label: "Medium" },
  { id: "high", label: "High" },
  { id: "xhigh", label: "XHigh" }
];

type PaneSettingsState = Record<string, PaneSettings>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isModelId(value: unknown): value is OpenAiModelId {
  return typeof value === "string" && openAiModelOptions.some((option) => option.id === value);
}

function isReasoningLevel(value: unknown): value is ReasoningLevel {
  return typeof value === "string" && reasoningOptions.some((option) => option.id === value);
}

function readSettingsState(): PaneSettingsState {
  try {
    const rawState = window.localStorage.getItem(paneSettingsKey);
    if (!rawState) {
      return {};
    }

    const parsed = JSON.parse(rawState);
    return isRecord(parsed) ? (parsed as PaneSettingsState) : {};
  } catch {
    return {};
  }
}

function writeSettingsState(state: PaneSettingsState): void {
  window.localStorage.setItem(paneSettingsKey, JSON.stringify(state));
}

function settingsFromMetadata(pane: PaneDefinition): Partial<PaneSettings> {
  if (!pane.metadataJson) {
    return {};
  }

  try {
    const metadata = JSON.parse(pane.metadataJson);
    if (!isRecord(metadata)) {
      return {};
    }

    return {
      modelId: isModelId(metadata.modelId) ? metadata.modelId : undefined,
      reasoningLevel: isReasoningLevel(metadata.reasoningLevel)
        ? metadata.reasoningLevel
        : undefined
    };
  } catch {
    return {};
  }
}

export function paneSettingsFor(pane: PaneDefinition): PaneSettings {
  const savedSettings = readSettingsState()[pane.id];
  const metadataSettings = settingsFromMetadata(pane);

  return {
    modelId:
      savedSettings?.modelId ??
      metadataSettings.modelId ??
      (isModelId(pane.modelId) ? pane.modelId : defaultModelId),
    reasoningLevel:
      savedSettings?.reasoningLevel ??
      metadataSettings.reasoningLevel ??
      defaultReasoningLevel
  };
}

export function updatePaneSettings(
  pane: PaneDefinition,
  patch: Partial<PaneSettings>
): PaneSettings {
  const currentSettings = paneSettingsFor(pane);
  const nextSettings = {
    modelId: patch.modelId ?? currentSettings.modelId,
    reasoningLevel: patch.reasoningLevel ?? currentSettings.reasoningLevel
  };

  writeSettingsState({
    ...readSettingsState(),
    [pane.id]: nextSettings
  });

  return nextSettings;
}
