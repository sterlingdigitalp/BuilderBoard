import type { ReactNode } from "react";
import type { AccountDto } from "../../types/accounts";
import type { EffortLevel, ModelId } from "../../types/paneSettings";
import type { ProjectDto } from "../../types/projects";
import type { EngineInfo } from "../../stores/engineCommands";
import type { BuilderInfo } from "../../stores/builderCommands";
import { defaultEffortOptions } from "../../stores/paneSettingsStore";

export const NEW_PROJECT_VALUE = "__new_project__";

interface ChatControlsProps {
  builders: BuilderInfo[];
  selectedBuilderId: string;
  engines: EngineInfo[];
  selectedEngineId: string;
  accounts: AccountDto[];
  selectedAccountId: string;
  selectedModelId: ModelId;
  selectedEffort: EffortLevel;
  projects: ProjectDto[];
  project: ProjectDto | null;
  disabled: boolean;
  builderError: string | null;
  engineError: string | null;
  accountError: string | null;
  statusSlot: ReactNode;
  onSelectBuilder: (builderName: string) => void;
  onSelectEngine: (engineId: string) => void;
  onSelectAccount: (accountId: string) => void;
  onSelectModel: (modelId: ModelId) => void;
  onSelectEffort: (effort: EffortLevel) => void;
  onSelectProject: (projectId: string) => void;
  onCreateProject: () => void;
}

function titleCase(value: string): string {
  if (value.length === 0) {
    return "";
  }

  return `${value[0].toUpperCase()}${value.slice(1)}`;
}

function effortLabel(effort: string): string {
  return effort === "max" ? "Max" : titleCase(effort);
}

function accountOptionLabel(account: AccountDto): string {
  const authType = account.authType === "oauth" ? "OAuth" : "API";
  const label = account.label.trim();

  if (account.isDefault) {
    return `Default ${authType}`;
  }

  return label.length > 0 ? `${label} ${authType}` : authType;
}

function projectTooltip(project: ProjectDto | null): string {
  if (!project) {
    return "No project\nNo approved root";
  }

  return `${project.name}\n${project.approvedRoot}`;
}

export function ChatControls({
  builders,
  selectedBuilderId,
  engines,
  selectedEngineId,
  accounts,
  selectedAccountId,
  selectedModelId,
  selectedEffort,
  projects,
  project,
  disabled,
  builderError,
  engineError,
  accountError,
  statusSlot,
  onSelectBuilder,
  onSelectEngine,
  onSelectAccount,
  onSelectModel,
  onSelectEffort,
  onSelectProject,
  onCreateProject
}: ChatControlsProps) {
  const selectedProjectId = project?.id ?? "";
  const currentEngine = engines.find((engine) => engine.id === selectedEngineId) || engines[0];
  const modelOptions = currentEngine
    ? currentEngine.models.map((model) => ({ id: model, label: model }))
    : [];
  const effortOptions = currentEngine
    ? currentEngine.supportedEfforts.map((effort) => ({ id: effort as EffortLevel, label: effort }))
    : defaultEffortOptions;

  return (
    <div className="chat-controls" aria-label="Pane controls">
      <label className="chat-controls__field chat-controls__field--project" title={projectTooltip(project)}>
        <select
          className="chat-controls__select chat-controls__select--project"
          value={selectedProjectId}
          disabled={disabled || projects.length === 0}
          onChange={(event) => {
            const nextValue = event.target.value;
            if (nextValue === NEW_PROJECT_VALUE) {
              onCreateProject();
              return;
            }
            onSelectProject(nextValue);
          }}
          aria-label={`Project ${project?.name ?? "not attached"}`}
        >
          {projects.length === 0 ? (
            <option value="">No Project</option>
          ) : (
            <>
              {!selectedProjectId ? <option value="">Select Project</option> : null}
              {projects.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.name}
                </option>
              ))}
              <option value={NEW_PROJECT_VALUE}>Add Project...</option>
            </>
          )}
        </select>
      </label>

      <label className="chat-controls__field chat-controls__field--builder">
        <select
          className="chat-controls__select chat-controls__select--builder"
          value={selectedBuilderId}
          disabled={disabled || builders.length === 0}
          onChange={(event) => onSelectBuilder(event.target.value)}
          aria-label="Builder"
        >
          {builderError ? (
            <option value={selectedBuilderId}>{builderError}</option>
          ) : builders.length === 0 ? (
            <option value="">No Builders Available</option>
          ) : (
            builders.map((builder) => (
              <option key={builder.name} value={builder.name}>
                {builder.displayName}
              </option>
            ))
          )}
        </select>
      </label>

      <span className="chat-controls__status-slot">{statusSlot}</span>

      <label className="chat-controls__field chat-controls__field--model">
        <select
          className="chat-controls__select"
          value={selectedModelId}
          disabled={disabled || Boolean(engineError) || modelOptions.length === 0}
          onChange={(event) => onSelectModel(event.target.value as ModelId)}
          aria-label="Model"
          title={selectedModelId}
        >
          {engineError ? (
            <option value={selectedModelId}>Model unavailable</option>
          ) : modelOptions.length === 0 ? (
            <option value={selectedModelId}>No Models Available</option>
          ) : (
            modelOptions.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label}
              </option>
            ))
          )}
        </select>
      </label>

      <label className="chat-controls__field chat-controls__field--effort">
        <select
          className="chat-controls__select"
          value={selectedEffort}
          disabled={disabled || effortOptions.length === 0}
          onChange={(event) => onSelectEffort(event.target.value as EffortLevel)}
          aria-label="Effort"
        >
          {effortOptions.map((option) => (
            <option key={option.id} value={option.id}>
              {effortLabel(option.label)}
            </option>
          ))}
        </select>
      </label>

      <label className="chat-controls__field chat-controls__field--engine">
        <select
          className="chat-controls__select"
          value={selectedEngineId}
          disabled={disabled || Boolean(engineError) || engines.length === 0}
          onChange={(event) => onSelectEngine(event.target.value)}
          aria-label="Execution engine"
          title={engineError ?? "Execution engine"}
        >
          {engineError ? (
            <option value={selectedEngineId}>Engine discovery failed</option>
          ) : engines.length === 0 ? (
            <option value="">No Engines Available</option>
          ) : (
            engines.map((engine) => (
              <option key={engine.id} value={engine.id}>
                {engine.displayName}
              </option>
            ))
          )}
        </select>
      </label>

      <label className="chat-controls__field chat-controls__field--account">
        <select
          className="chat-controls__select"
          value={selectedAccountId}
          disabled={disabled || Boolean(accountError) || accounts.length === 0 || selectedEngineId === "grok"}
          onChange={(event) => onSelectAccount(event.target.value)}
          aria-label="Account"
          title={accountError ?? "Account"}
        >
          {accountError ? (
            <option value={selectedAccountId}>No accounts available</option>
          ) : accounts.length === 0 ? (
            <option value="">No Account</option>
          ) : (
            accounts.map((account) => (
              <option key={account.id} value={account.id}>
                {accountOptionLabel(account)}
              </option>
            ))
          )}
        </select>
      </label>
    </div>
  );
}
