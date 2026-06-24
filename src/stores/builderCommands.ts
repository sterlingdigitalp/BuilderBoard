import { invoke } from "@tauri-apps/api/core";

export interface BuilderExecution {
  preferredEngine: string;
  fallbackEngines: string[];
  effort: string;
  defaultModel: string;
  reviewRequirements: string;
  memoryDefaults: string;
}

export interface BuilderInfo {
  name: string;
  displayName: string;
  execution: BuilderExecution;
}

export async function builderList(): Promise<BuilderInfo[]> {
  return await invoke<BuilderInfo[]>("builder_list");
}
