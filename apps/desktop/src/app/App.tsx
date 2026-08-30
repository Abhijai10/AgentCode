import { useEffect, useState, useCallback } from "react";
import { ThemeProvider } from "./ThemeContext";
import { ProjectProvider, useProject, type Project } from "./ProjectContext";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import { Footer } from "./Footer";
import { HomeView } from "./HomeView";
import { MissionView } from "./MissionView";
import { DiscussView } from "./DiscussView";
import { DesignView } from "./DesignView";
import { SecurityView } from "./SecurityView";
import { SettingsView } from "./SettingsView";
import { ProjectModal } from "./ProjectModal";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { View, MissionSummary, DaemonStatus } from "./types";

const PROJECT_REQUIRED_VIEWS: View[] = ["mission", "discuss", "design", "security"];

function AppShell() {
  const [view, setView] = useState<View>("home");
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus | null>(null);
  const [missions, setMissions] = useState<MissionSummary[]>([]);
  const [activeMission, setActiveMission] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [projectModal, setProjectModal] = useState<null | "open" | "new" | "choose">(null);
  const [projectAlert, setProjectAlert] = useState(false);
  const [routingProfile, setRoutingProfile] = useState("free_first");
  const { project, openProject } = useProject();

  const requestView = useCallback(
    (target: View) => {
      if (!project && PROJECT_REQUIRED_VIEWS.includes(target)) {
        setProjectAlert(true);
        return;
      }
      setView(target);
    },
    [project]
  );

  useEffect(() => {
    let cancelled = false;
    const refresh = async () => {
      const [d, m, s] = await Promise.all([daemon.health(), daemon.listActiveMissions(), daemon.getSettings()]);
      if (cancelled) return;
      setDaemonStatus(d);
      setMissions(m);
      setRoutingProfile(s?.routing_profile ?? "free_first");
      if (!activeMission && m.length > 0) setActiveMission(m[0].mission_id);
    };
    refresh();
    const timer = setInterval(refresh, 4000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [activeMission]);

  const handleOpenedProject = (p: Project) => {
    openProject(p.path, p.name);
    setProjectModal(null);
    setProjectAlert(false);
    setView("home");
    setNotice(`Project "${p.name}" opened.`);
    setTimeout(() => setNotice(null), 4000);
  };

  const handleNewMission = (goal: string) => {
    if (!project) {
      setProjectModal("choose");
      return;
    }
    const doSubmit = async () => {
      const created = await daemon.submitMission(goal, project?.path);
      if (created.ok) {
        setActiveMission(created.mission_id);
        setView("mission");
        const m = await daemon.listActiveMissions();
        setMissions(m);
        setNotice("Mission submitted to the AgentCode daemon.");
        setTimeout(() => setNotice(null), 4000);
      } else {
        // Surface the real backend error (e.g. a rejected workspace_root)
        // instead of a fabricated success or a generic failure message.
        setNotice(created.error || "Daemon is not connected — could not submit mission.");
        setTimeout(() => setNotice(null), 6000);
      }
    };
    doSubmit();
  };

  const daemonConnected = daemonStatus?.state === "running";

  return (
    <div className="h-screen w-screen flex flex-col bg-background text-on-surface overflow-hidden font-body antialiased dark:bg-background dark:text-on-surface">
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          view={view}
          setView={requestView}
          daemonConnected={daemonConnected}
          project={project}
          onOpenProject={() => setProjectModal(project ? "open" : "choose")}
        />
        <div className="flex-1 flex flex-col overflow-hidden">
          <TopBar
            project={project ? `Project: ${project.name}` : "No Project Open"}
            branch={project ? "main" : "—"}
            onOpenProject={() => setProjectModal(project ? "open" : "choose")}
            onNewMission={() => {
              if (project) {
                setView("home");
              } else {
                setProjectModal("choose");
              }
            }}
          />
          {view === "home" && (
            <HomeView
              project={project}
              onOpenProject={() => setProjectModal("choose")}
              onSubmit={handleNewMission}
              onConfigureProviders={() => setView("settings")}
            />
          )}
          {view === "mission" && <MissionView missionId={activeMission} onOpenSettings={() => setView("settings")} />}
          {view === "discuss" && <DiscussView project={project?.name ?? "AgentCode"} branch="main" />}
          {view === "design" && <DesignView />}
          {view === "security" && <SecurityView />}
          {view === "settings" && <SettingsView />}
          <Footer
            daemonConnected={daemonConnected}
            modelLabel={`Routing: ${routingProfile.replace(/_/g, " ")}`}
            tasks={`Tasks: ${missions.length}`}
          />
        </div>
      </div>

      {projectModal && (
        <ProjectModal
          mode={projectModal}
          onClose={() => setProjectModal(null)}
          onOpened={handleOpenedProject}
        />
      )}

      {projectAlert && (
        <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={() => setProjectAlert(false)}>
          <div
            className="bg-surface rounded-2xl p-8 max-w-sm w-full mx-4 neo-raised"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex flex-col items-center text-center gap-4">
              <div className="w-12 h-12 rounded-full neo-pressed flex items-center justify-center text-primary">
                <Icon name="folder_open" size={24} />
              </div>
              <h3 className="text-lg font-semibold text-on-surface">Open a Project First</h3>
              <p className="text-sm text-on-surface-variant">Please open a project to start working. You can open an existing one or create a new one.</p>
              <div className="flex gap-3 mt-2">
                <button
                  onClick={() => {
                    setProjectAlert(false);
                    setProjectModal("choose");
                  }}
                  className="neo-button px-5 py-3 rounded-xl text-sm text-primary font-medium"
                >
                  Open Project
                </button>
                <button
                  onClick={() => setProjectAlert(false)}
                  className="neo-button px-5 py-3 rounded-xl text-sm text-on-surface-variant font-medium"
                >
                  Dismiss
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {notice && (
        <div className="fixed bottom-10 left-1/2 -translate-x-1/2 z-50 neo-raised rounded-xl px-5 py-3 text-sm font-medium text-on-surface">
          {notice}
        </div>
      )}
    </div>
  );
}

export function App() {
  return (
    <ThemeProvider>
      <ProjectProvider>
        <AppShell />
      </ProjectProvider>
    </ThemeProvider>
  );
}