import type { PaneDefinition } from "../types/layout";
import type {
  EffortLevel,
  EffortOption,
  EngineId,
  ModelId,
  ModelOption,
  PaneSettings
} from "../types/paneSettings";

const paneSettingsKey = "builderboard.paneSettings.v1";
const defaultEngineId: EngineId = "openai";
const defaultModelId: ModelId = "GPT-5.5";
const defaultEffort: EffortLevel = "medium";

export const defaultEngineOptions: { id: EngineId; label: string }[] = []; // populated dynamically

export const defaultEffortOptions: EffortOption[] = [
  { id: "low", label: "Low" },
  { id: "medium", label: "Medium" },
  { id: "high", label: "High" },
  { id: "max", label: "Max" }
];

type PaneSettingsState = Record<string, PaneSettings>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isEngineId(value: unknown): value is EngineId {
  return typeof value === "string";
}

function isModelId(value: unknown): value is ModelId {
  return typeof value === "string";
}

function isEffortLevel(value: unknown): value is EffortLevel {
  return typeof value === "string" && defaultEffortOptions.some((option) => option.id === value);
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
      engineId: isEngineId(metadata.engineId) ? metadata.engineId : undefined,
      modelId: isModelId(metadata.modelId) ? metadata.modelId : undefined,
      effort: isEffortLevel(metadata.effort) ? metadata.effort : undefined
    };
  } catch {
    return {};
  }
}

export function paneSettingsFor(pane: PaneDefinition): PaneSettings {
  const savedSettings = readSettingsState()[pane.id];
  const metadataSettings = settingsFromMetadata(pane);

  return {
    engineId:
      savedSettings?.engineId ??
      metadataSettings.engineId ??
      (isEngineId(pane.providerId) ? pane.providerId : defaultEngineId),
    modelId:
      savedSettings?.modelId ??
      metadataSettings.modelId ??
      (isModelId(pane.modelId) ? pane.modelId : defaultModelId),
    effort:
      savedSettings?.effort ??
      metadataSettings.effort ??
      defaultEffort
  };
}

export function updatePaneSettings(
  pane: PaneDefinition,
  patch: Partial<PaneSettings>
): PaneSettings {
  const currentSettings = paneSettingsFor(pane);
  const nextSettings = {
    engineId: patch.engineId ?? currentSettings.engineId,
    modelId: patch.modelId ?? currentSettings.modelId,
    effort: patch.effort ?? currentSettings.effort
  };

  writeSettingsState({
    ...readSettingsState(),
    [pane.id]: nextSettings
  });

  return nextSettings;
}
