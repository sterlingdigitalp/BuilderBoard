import type { CSSProperties } from "react";
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
  onSelectBuilder: (builderName: string) => void;
  onSelectEngine: (engineId: string) => void;
  onSelectAccount: (accountId: string) => void;
  onSelectModel: (modelId: ModelId) => void;
  onSelectEffort: (effort: EffortLevel) => void;
  onSelectProject: (projectId: string) => void;
  onCreateProject: () => void;
}

const rowStyle: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "18fr 18fr 28fr 18fr 18fr",
  gap: 3,
  minWidth: 0,
  flex: "1 1 auto"
};

const fieldStyle: CSSProperties = {
  display: "block",
  minWidth: 0
};

const selectStyle: CSSProperties = {
  width: "100%",
  minWidth: 0,
  height: 24,
  border: "1px solid var(--pane-border)",
  borderRadius: 5,
  background: "var(--button-bg)",
  color: "var(--text-strong)",
  font: "inherit",
  fontSize: "0.68rem",
  padding: "0 2px"
};

const projectSelectStyle: CSSProperties = {
  ...selectStyle,
  color: "var(--button-active-bg)",
  fontSize: "0.7rem",
  fontWeight: 700,
  padding: "0 5px"
};

function accountOptionLabel(account: AccountDto): string {
  const authType = account.authType === "oauth" ? "OAuth" : "API";
  return account.isDefault ? `${authType} *` : authType;
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
  onSelectBuilder,
  onSelectEngine,
  onSelectAccount,
  onSelectModel,
  onSelectEffort,
  onSelectProject,
  onCreateProject
}: ChatControlsProps) {
  const selectedProjectId = project?.id ?? "";
  const currentEngine = engines.find((e) => e.id === selectedEngineId) || engines[0];
  const modelOptions = currentEngine ? currentEngine.models.map(m => ({id: m, label: m})) : [];
  const effortOptions = currentEngine ? currentEngine.supportedEfforts.map(e => ({id: e as any, label: e})) : defaultEffortOptions;

  return (
    <div style={rowStyle} aria-label="Chat controls">
      <label style={fieldStyle}>
        <select
          style={selectStyle}
          value={selectedBuilderId}
          disabled={disabled || builders.length === 0}
          onChange={(event) => onSelectBuilder(event.target.value)}
          aria-label="Builder"
        >
          {builders.length === 0 ? (
            <option value="">No builders</option>
          ) : (
            builders.map((b) => (
              <option key={b.name} value={b.name}>
                {b.displayName}
              </option>
            ))
          )}
        </select>
      </label>

      <label style={fieldStyle}>
        <select
          style={selectStyle}
          value={selectedEngineId}
          disabled={disabled || engines.length === 0}
          onChange={(event) => onSelectEngine(event.target.value)}
          aria-label="Engine"
        >
          {engines.length === 0 ? (
            <option value="">No engines</option>
          ) : (
            engines.map((eng) => (
              <option key={eng.id} value={eng.id}>
                {eng.displayName} ({eng.health})
              </option>
            ))
          )}
        </select>
      </label>

      <label style={fieldStyle}>
        <select
          style={selectStyle}
          value={selectedAccountId}
          disabled={disabled || accounts.length === 0 || selectedEngineId === "grok"}
          onChange={(event) => onSelectAccount(event.target.value)}
          aria-label="Account"
        >
          {accounts.length === 0 ? (
            <option value="">None</option>
          ) : (
            accounts.map((account) => (
              <option key={account.id} value={account.id}>
                {accountOptionLabel(account)}
              </option>
            ))
          )}
        </select>
      </label>

      <label style={fieldStyle} title={projectTooltip(project)}>
        <select
          style={projectSelectStyle}
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
            <option value="">No project</option>
          ) : (
            <>
              {!selectedProjectId ? <option value="">Select project</option> : null}
              {projects.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.name}
                </option>
              ))}
              <option value={NEW_PROJECT_VALUE}>* New Project</option>
            </>
          )}
        </select>
      </label>

      <label style={fieldStyle}>
        <select
          style={selectStyle}
          value={selectedModelId}
          disabled={disabled || modelOptions.length === 0}
          onChange={(event) => onSelectModel(event.target.value as ModelId)}
          aria-label="Model"
        >
          {modelOptions.map((option) => (
            <option key={option.id} value={option.id}>
              {option.label}
            </option>
          ))}
        </select>
      </label>

      <label style={fieldStyle}>
        <select
          style={selectStyle}
          value={selectedEffort}
          disabled={disabled || effortOptions.length === 0}
          onChange={(event) => onSelectEffort(event.target.value as EffortLevel)}
          aria-label="Effort"
        >
          {effortOptions.map((option) => (
            <option key={option.id} value={option.id}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}