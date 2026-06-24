import { useCallback, useEffect, useMemo, useState } from "react";
import {
  PROJECT_CHANGED_EVENT,
  projectList,
  readActiveProjectId
} from "../stores/projectCommands";
import type { ProjectDto } from "../types/projects";

interface ProjectState {
  projects: ProjectDto[];
  activeProjectId: string;
  activeProject: ProjectDto | null;
  isLoading: boolean;
  isMutating: boolean;
  error: string | null;
  launcherProjectId: string;
  setLauncherProject: (projectId: string) => void;
  reloadProjects: (options?: { silent?: boolean }) => Promise<void>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Project command failed.";
}

export function useProjects(): ProjectState {
  const [projects, setProjects] = useState<ProjectDto[]>([]);
  const [activeProjectId, setActiveProjectId] = useState(readActiveProjectId);
  const [isLoading, setIsLoading] = useState(true);
  const [isMutating, setIsMutating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeProject = useMemo(
    () => projects.find((project) => project.id === activeProjectId) ?? null,
    [activeProjectId, projects]
  );

  const reloadProjects = useCallback(async (options?: { silent?: boolean }) => {
    const silent = options?.silent === true;
    if (!silent) {
      setIsLoading(true);
    }
    setError(null);

    try {
      const loadedProjects = await projectList();
      const nextActiveProjectId = readActiveProjectId();

      setProjects(loadedProjects);
      setActiveProjectId(
        loadedProjects.some((project) => project.id === nextActiveProjectId)
          ? nextActiveProjectId
          : loadedProjects.find((project) => project.isActive)?.id ??
              loadedProjects[0]?.id ??
              nextActiveProjectId
      );
    } catch (loadError) {
      setError(errorMessage(loadError));
    } finally {
      if (!silent) {
        setIsLoading(false);
      }
    }
  }, []);

  const setLauncherProject = useCallback((projectId: string) => {
    setActiveProjectId(projectId);
  }, []);

  useEffect(() => {
    void reloadProjects();
  }, [reloadProjects]);

  useEffect(() => {
    function handleProjectChange() {
      void reloadProjects({ silent: true });
    }

    window.addEventListener(PROJECT_CHANGED_EVENT, handleProjectChange);
    window.addEventListener("storage", handleProjectChange);

    return () => {
      window.removeEventListener(PROJECT_CHANGED_EVENT, handleProjectChange);
      window.removeEventListener("storage", handleProjectChange);
    };
  }, [reloadProjects]);

  return {
    projects,
    activeProjectId,
    activeProject,
    isLoading,
    isMutating,
    error,
    launcherProjectId: activeProjectId,
    setLauncherProject,
    reloadProjects
  };
}