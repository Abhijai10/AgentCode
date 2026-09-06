import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

export interface Project {
  path: string;
  name: string;
}

interface ProjectContextValue {
  project: Project | null;
  openProject: (path: string, name?: string) => void;
  clearProject: () => void;
}

const ProjectContext = createContext<ProjectContextValue>({
  project: null,
  openProject: () => {},
  clearProject: () => {},
});

function nameFromPath(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] || path;
}

function loadStoredProject(): Project | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = localStorage.getItem("agentcode-project");
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Project;
    if (parsed && typeof parsed.path === "string" && parsed.path) return parsed;
    return null;
  } catch {
    return null;
  }
}

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [project, setProject] = useState<Project | null>(loadStoredProject);

  useEffect(() => {
    if (project) {
      localStorage.setItem("agentcode-project", JSON.stringify(project));
    } else {
      localStorage.removeItem("agentcode-project");
    }
  }, [project]);

  const openProject = (path: string, name?: string) => {
    setProject({ path, name: name || nameFromPath(path) });
  };

  const clearProject = () => setProject(null);

  return (
    <ProjectContext.Provider value={{ project, openProject, clearProject }}>
      {children}
    </ProjectContext.Provider>
  );
}

export function useProject() {
  return useContext(ProjectContext);
}