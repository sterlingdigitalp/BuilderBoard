import { setShellView } from "../../hooks/useShellView";
import type { ProjectDto } from "../../types/projects";

interface ProjectRailProps {
  projects: ProjectDto[];
  activeProjectId: string;
  isLoading: boolean;
  isMutating: boolean;
  onCreateProject: () => void;
  onLaunchProject: (projectId: string) => void | Promise<void>;
}

export function ProjectRail({
  projects,
  activeProjectId,
  isLoading,
  isMutating,
  onCreateProject,
  onLaunchProject
}: ProjectRailProps) {
  return (
    <nav className="sidebar__projects" aria-label="Projects">
      <div className="sidebar__projects-scroll">
        {projects.map((project) => (
          <button
            key={project.id}
            className={`sidebar__project-button${
              project.id === activeProjectId ? " sidebar__project-button--active" : ""
            }`}
            type="button"
            aria-label={`Launch pane for ${project.name}`}
            title={project.name}
            disabled={isLoading || isMutating}
            onClick={() => {
              void onLaunchProject(project.id);
              setShellView("board");
            }}
          >
            <span aria-hidden="true">{project.code}</span>
          </button>
        ))}
        <button
          className="sidebar__project-button sidebar__project-button--create"
          type="button"
          aria-label="Add project from folder"
          title="Add project"
          disabled={isLoading || isMutating}
          onClick={onCreateProject}
        >
          <span aria-hidden="true">＋</span>
        </button>
      </div>
    </nav>
  );
}