export type EngineId = string;
export type ModelId = string;
export type EffortLevel = "low" | "medium" | "high" | "max";

export interface PaneSettings {
  engineId: EngineId;
  modelId: ModelId;
  effort: EffortLevel;
}

export interface EngineOption {
  id: EngineId;
  label: string;
}

export interface ModelOption {
  id: ModelId;
  label: string;
}

export interface EffortOption {
  id: EffortLevel;
  label: string;
}
