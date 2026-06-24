import type { CSSProperties } from "react";
import type { AccountDto } from "../../types/accounts";
import type { OpenAiModelId, ReasoningLevel } from "../../types/paneSettings";
import type { ProjectDto } from "../../types/projects";
import { openAiModelOptions, reasoningOptions } from "../../stores/paneSettingsStore";

export const NEW_PROJECT_VALUE = "__new_project__";

interface ChatControlsProps {
  accounts: AccountDto[];
  selectedAccountId: string;
  selectedModelId: OpenAiModelId;
  selectedReasoningLevel: ReasoningLevel;
  projects: ProjectDto[];
  project: ProjectDto | null;
  disabled: boolean;
  onSelectAccount: (accountId: string) => void;
  onSelectModel: (modelId: OpenAiModelId) => void;
  onSelectReasoning: (reasoningLevel: ReasoningLevel) => void;
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
  accounts,
  selectedAccountId,
  selectedModelId,
  selectedReasoningLevel,
  projects,
  project,
  disabled,
  onSelectAccount,
  onSelectModel,
  onSelectReasoning,
  onSelectProject,
  onCreateProject
}: ChatControlsProps) {
  const selectedProjectId = project?.id ?? "";

  return (
    <div style={rowStyle} aria-label="Chat controls">
      <label style={fieldStyle}>
        <select style={selectStyle} value="openai" disabled={disabled} aria-label="Provider">
          <option value="openai">OpenAI</option>
          <option value="anthropic" disabled>
            Anthropic
          </option>
          <option value="google" disabled>
            Google
          </option>
        </select>
      </label>

      <label style={fieldStyle}>
        <select
          style={selectStyle}
          value={selectedAccountId}
          disabled={disabled || accounts.length === 0}
          onChange={(event) => onSelectAccount(event.target.value)}
          aria-label="OpenAI account"
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
          disabled={disabled}
          onChange={(event) => onSelectModel(event.target.value as OpenAiModelId)}
          aria-label="OpenAI model"
        >
          {openAiModelOptions.map((option) => (
            <option key={option.id} value={option.id}>
              {option.label}
            </option>
          ))}
        </select>
      </label>

      <label style={fieldStyle}>
        <select
          style={selectStyle}
          value={selectedReasoningLevel}
          disabled={disabled}
          onChange={(event) => onSelectReasoning(event.target.value as ReasoningLevel)}
          aria-label="Reasoning level"
        >
          {reasoningOptions.map((option) => (
            <option key={option.id} value={option.id}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}