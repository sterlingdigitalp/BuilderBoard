export type OpenAiModelId = "gpt-5.5" | "gpt-5.4-mini" | "gpt-5.3-codex-spark";
export type ReasoningLevel = "low" | "medium" | "high" | "xhigh";

export interface PaneSettings {
  modelId: OpenAiModelId;
  reasoningLevel: ReasoningLevel;
}

export interface ModelOption {
  id: OpenAiModelId;
  label: string;
}

export interface ReasoningOption {
  id: ReasoningLevel;
  label: string;
}
