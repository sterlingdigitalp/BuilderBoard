import { useCallback, useEffect, useMemo, useState } from "react";
import {
  readActiveWorkspaceId,
  WORKSPACE_CHANGED_EVENT,
  workspaceCreate,
  workspaceList,
  workspaceSwitch
} from "../stores/workspaceCommands";
import type { WorkspaceDto } from "../types/workspaces";

interface WorkspaceState {
  workspaces: WorkspaceDto[];
  activeWorkspaceId: string;
  activeWorkspace: WorkspaceDto | null;
  isLoading: boolean;
  isMutating: boolean;
  error: string | null;
  createWorkspace: (name: string) => Promise<void>;
  switchWorkspace: (workspaceId: string) => Promise<void>;
  reloadWorkspaces: (options?: { silent?: boolean }) => Promise<void>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Workspace command failed.";
}

export function useWorkspaces(): WorkspaceState {
  const [workspaces, setWorkspaces] = useState<WorkspaceDto[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState(readActiveWorkspaceId);
  const [isLoading, setIsLoading] = useState(true);
  const [isMutating, setIsMutating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeWorkspace = useMemo(
    () => workspaces.find((workspace) => workspace.id === activeWorkspaceId) ?? null,
    [activeWorkspaceId, workspaces]
  );

  const reloadWorkspaces = useCallback(async (options?: { silent?: boolean }) => {
    const silent = options?.silent === true;
    if (!silent) {
      setIsLoading(true);
    }
    setError(null);

    try {
      const loadedWorkspaces = await workspaceList();
      const nextActiveWorkspaceId = readActiveWorkspaceId();

      setWorkspaces(loadedWorkspaces);
      setActiveWorkspaceId(
        loadedWorkspaces.some((workspace) => workspace.id === nextActiveWorkspaceId)
          ? nextActiveWorkspaceId
          : loadedWorkspaces[0]?.id ?? nextActiveWorkspaceId
      );
    } catch (loadError) {
      setError(errorMessage(loadError));
    } finally {
      if (!silent) {
        setIsLoading(false);
      }
    }
  }, []);

  const createWorkspace = useCallback(async (name: string) => {
    setIsMutating(true);
    setError(null);

    try {
      const createdWorkspace = await workspaceCreate({ name });
      setActiveWorkspaceId(createdWorkspace.id);
      await reloadWorkspaces({ silent: true });
    } catch (createError) {
      setError(errorMessage(createError));
    } finally {
      setIsMutating(false);
    }
  }, [reloadWorkspaces]);

  const switchWorkspace = useCallback(async (workspaceId: string) => {
    setIsMutating(true);
    setError(null);

    try {
      await workspaceSwitch(workspaceId);
      setActiveWorkspaceId(workspaceId);
    } catch (switchError) {
      setError(errorMessage(switchError));
    } finally {
      setIsMutating(false);
    }
  }, []);

  useEffect(() => {
    void reloadWorkspaces();
  }, [reloadWorkspaces]);

  useEffect(() => {
    function handleWorkspaceChange() {
      void reloadWorkspaces({ silent: true });
    }

    window.addEventListener(WORKSPACE_CHANGED_EVENT, handleWorkspaceChange);
    window.addEventListener("storage", handleWorkspaceChange);

    return () => {
      window.removeEventListener(WORKSPACE_CHANGED_EVENT, handleWorkspaceChange);
      window.removeEventListener("storage", handleWorkspaceChange);
    };
  }, [reloadWorkspaces]);

  return {
    workspaces,
    activeWorkspaceId,
    activeWorkspace,
    isLoading,
    isMutating,
    error,
    createWorkspace,
    switchWorkspace,
    reloadWorkspaces
  };
}
