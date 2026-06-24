import { useCallback, useEffect, useRef, useState } from "react";
import { usePersistentPanes } from "./usePersistentPanes";
import { useProjects } from "./useProjects";
import { paneCreate, paneList } from "../stores/paneCommands";
import { DEFAULT_WORKSPACE_ID } from "../stores/workspaceCommands";
import {
  pickProjectFolder,
  projectCreateFromFolder
} from "../stores/projectCommands";
import type { ProjectDto } from "../types/projects";

export function usePaneWorkspace() {
  const projectsState = useProjects();
  const {
    panes,
    isLoading,
    isMutating,
    error,
    createPane,
    closePane,
    reloadPanes,
    setPaneProject
  } = usePersistentPanes(DEFAULT_WORKSPACE_ID, projectsState.launcherProjectId);
  const [focusedPaneId, setFocusedPaneId] = useState<string | null>(null);
  const paneRefs = useRef<Record<string, HTMLElement | null>>({});

  const focusPane = useCallback((paneId: string) => {
    setFocusedPaneId(paneId);
    const element = paneRefs.current[paneId];
    element?.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
  }, []);

  const registerPaneRef = useCallback((paneId: string, element: HTMLElement | null) => {
    if (element) {
      paneRefs.current[paneId] = element;
    } else {
      delete paneRefs.current[paneId];
    }
  }, []);

  useEffect(() => {
    if (focusedPaneId === null || panes.some((pane) => pane.id === focusedPaneId)) {
      return;
    }

    setFocusedPaneId(null);
  }, [focusedPaneId, panes]);

  const createPaneAndFocus = useCallback(async () => {
    const createdPane = await createPane();
    if (createdPane) {
      focusPane(createdPane.id);
    }
  }, [createPane, focusPane]);

  const closePaneAndTransferFocus = useCallback(
    async (paneId: string) => {
      const currentIndex = panes.findIndex((pane) => pane.id === paneId);
      const remainingPanes = panes.filter((pane) => pane.id !== paneId);
      const nextFocusedPane =
        currentIndex >= 0
          ? remainingPanes[Math.min(currentIndex, remainingPanes.length - 1)] ?? null
          : null;

      const didClose = await closePane(paneId);

      if (didClose) {
        if (nextFocusedPane) {
          focusPane(nextFocusedPane.id);
        } else {
          setFocusedPaneId(null);
        }
      }
    },
    [closePane, focusPane, panes]
  );

  const launchProjectPane = useCallback(
    async (projectId: string) => {
      const createdPane = await paneCreate(DEFAULT_WORKSPACE_ID, projectId);
      await reloadPanes();
      if (createdPane) {
        focusPane(createdPane.id);
      }
      projectsState.setLauncherProject(projectId);
    },
    [focusPane, projectsState, reloadPanes]
  );

  const createProjectAndLaunchPane = useCallback(async () => {
    const folderPath = await pickProjectFolder();
    if (folderPath === null) {
      return;
    }

    const createdProject = await projectCreateFromFolder(folderPath, true);
    await projectsState.reloadProjects({ silent: true });
    await reloadPanes();

    const refreshedPanes = await paneList(DEFAULT_WORKSPACE_ID);
    const launchedPane = refreshedPanes.find((pane) => pane.projectId === createdProject.id);
    if (launchedPane) {
      focusPane(launchedPane.id);
    } else {
      await launchProjectPane(createdProject.id);
    }
    projectsState.setLauncherProject(createdProject.id);
  }, [launchProjectPane, projectsState, reloadPanes, focusPane]);

  const bindPaneToNewProject = useCallback(
    async (paneId: string) => {
      const folderPath = await pickProjectFolder();
      if (folderPath === null) {
        return;
      }

      const createdProject = await projectCreateFromFolder(folderPath, false);
      await setPaneProject(paneId, createdProject.id);
      await projectsState.reloadProjects({ silent: true });
      projectsState.setLauncherProject(createdProject.id);
      focusPane(paneId);
    },
    [focusPane, projectsState, setPaneProject]
  );

  const changePaneProject = useCallback(
    async (paneId: string, projectId: string, projectList: ProjectDto[]) => {
      await setPaneProject(paneId, projectId);
      const project = projectList.find((entry) => entry.id === projectId);
      if (project) {
        projectsState.setLauncherProject(project.id);
      }
      focusPane(paneId);
    },
    [focusPane, projectsState, setPaneProject]
  );

  return {
    projects: projectsState.projects,
    launcherProjectId: projectsState.launcherProjectId,
    isLoadingProjects: projectsState.isLoading,
    projectError: projectsState.error,
    isProjectMutating: projectsState.isMutating,
    panes,
    isLoading,
    isMutating,
    error,
    focusedPaneId,
    focusPane,
    createPane: createPaneAndFocus,
    closePane: closePaneAndTransferFocus,
    launchProjectPane,
    createProjectAndLaunchPane,
    bindPaneToNewProject,
    changePaneProject,
    registerPaneRef
  };
}
