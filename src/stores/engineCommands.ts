import { invoke } from "@tauri-apps/api/core";

export interface EngineCapabilitiesFeatures {
  chat: boolean;
  streaming: boolean;
  reasoning: boolean;
  toolUse: boolean;
  images: boolean;
  embeddings: boolean;
  structuredOutput: boolean;
  multimodal: boolean;
  filesystem: boolean;
  shell: boolean;
  subagents: boolean;
  worktrees: boolean;
  cancellation: boolean;
}

export interface EngineCapabilities {
  locality: string;
  features: EngineCapabilitiesFeatures;
}

export interface EngineInfo {
  id: string;
  displayName: string;
  models: string[];
  supportedEfforts: string[];
  health: string;
  capabilities: EngineCapabilities;
}

export async function engineList(): Promise<EngineInfo[]> {
  return await invoke<EngineInfo[]>("engine_list");
}
