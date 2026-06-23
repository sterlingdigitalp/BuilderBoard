import { useCallback, useEffect, useState } from "react";
import { paneClose, paneCreate, paneList } from "../stores/paneCommands";
import type { PaneDto } from "../types/layout";

interface PersistentPaneState {
  panes: PaneDto[];
  isLoading: boolean;
  isMutating: boolean;
  error: string | null;
  createPane: () => Promise<void>;
  closePane: (paneId: string) => Promise<void>;
  reloadPanes: () => Promise<void>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Pane persistence command failed.";
}

export function usePersistentPanes(): PersistentPaneState {
  const [panes, setPanes] = useState<PaneDto[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isMutating, setIsMutating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reloadPanes = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      setPanes(await paneList());
    } catch (loadError) {
      setError(errorMessage(loadError));
      setPanes([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const createPane = useCallback(async () => {
    setIsMutating(true);
    setError(null);

    try {
      const createdPane = await paneCreate();

      if (createdPane) {
        setPanes((currentPanes) =>
          [...currentPanes, createdPane].sort((a, b) => a.sortOrder - b.sortOrder)
        );
      } else {
        setPanes(await paneList());
      }
    } catch (createError) {
      setError(errorMessage(createError));
    } finally {
      setIsMutating(false);
    }
  }, []);

  const closePane = useCallback(async (paneId: string) => {
    setIsMutating(true);
    setError(null);

    try {
      await paneClose(paneId);
      setPanes((currentPanes) => currentPanes.filter((pane) => pane.id !== paneId));
    } catch (closeError) {
      setError(errorMessage(closeError));
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
    reloadPanes
  };
}
