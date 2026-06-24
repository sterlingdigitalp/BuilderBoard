import { useCallback, useEffect, useState } from "react";
import { paneClose, paneCreate, paneList, paneSetProject } from "../stores/paneCommands";
import type { PaneDto } from "../types/layout";

interface PersistentPaneState {
  panes: PaneDto[];
  isLoading: boolean;
  isMutating: boolean;
  error: string | null;
  createPane: () => Promise<PaneDto | null>;
  closePane: (paneId: string) => Promise<boolean>;
  setPaneProject: (paneId: string, projectId: string) => Promise<void>;
  reloadPanes: () => Promise<void>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Pane persistence command failed.";
}

export function usePersistentPanes(
  workspaceId?: string,
  defaultProjectId?: string
): PersistentPaneState {
  const [panes, setPanes] = useState<PaneDto[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isMutating, setIsMutating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reloadPanes = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      setPanes(await paneList(workspaceId));
    } catch (loadError) {
      setError(errorMessage(loadError));
      setPanes([]);
    } finally {
      setIsLoading(false);
    }
  }, [workspaceId, defaultProjectId]);

  const createPane = useCallback(async (): Promise<PaneDto | null> => {
    setIsMutating(true);
    setError(null);
    const knownPaneIds = new Set(panes.map((pane) => pane.id));

    try {
      const createdPane = await paneCreate(workspaceId, defaultProjectId);

      if (createdPane) {
        setPanes((currentPanes) =>
          [...currentPanes, createdPane].sort((a, b) => a.sortOrder - b.sortOrder)
        );
      } else {
        const reloadedPanes = await paneList(workspaceId);
        setPanes(reloadedPanes);
        return reloadedPanes.find((pane) => !knownPaneIds.has(pane.id)) ?? null;
      }

      return createdPane;
    } catch (createError) {
      setError(errorMessage(createError));
      return null;
    } finally {
      setIsMutating(false);
    }
  }, [workspaceId, defaultProjectId, panes]);

  const closePane = useCallback(async (paneId: string): Promise<boolean> => {
    setIsMutating(true);
    setError(null);

    try {
      await paneClose(paneId);
      setPanes((currentPanes) => currentPanes.filter((pane) => pane.id !== paneId));
      return true;
    } catch (closeError) {
      setError(errorMessage(closeError));
      return false;
    } finally {
      setIsMutating(false);
    }
  }, []);

  const setPaneProjectBinding = useCallback(async (paneId: string, projectId: string) => {
    setIsMutating(true);
    setError(null);

    try {
      const updatedPane = await paneSetProject(paneId, projectId);
      setPanes((currentPanes) =>
        currentPanes.map((pane) => (pane.id === paneId ? updatedPane : pane))
      );
    } catch (bindError) {
      setError(errorMessage(bindError));
    } finally {
      setIsMutating(false);
    }
  }, []);

  useEffect(() => {
    void reloadPanes();
  }, [reloadPanes]);

  return {
    panes,
    isLoading,
    isMutating,
    error,
    createPane,
    closePane,
    setPaneProject: setPaneProjectBinding,
    reloadPanes
  };
}
