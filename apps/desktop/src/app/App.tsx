import { useEffect, useState, useCallback } from "react";
import { ThemeProvider } from "./ThemeContext";
import { ProjectProvider, useProject, type Project } from "./ProjectContext";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import { Footer } from "./Footer";
import { HomeView } from "./HomeView";
import { MissionView } from "./MissionView";
import { ChatView } from "./ChatView";
import { DiscussView } from "./DiscussView";
import { DesignView } from "./DesignView";
import { SecurityView } from "./SecurityView";
import { SettingsView } from "./SettingsView";
import { ProjectModal } from "./ProjectModal";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { View, MissionSummary, DaemonStatus } from "./types";

const PROJECT_REQUIRED_VIEWS: View[] = ["mission", "chat", "discuss", "design", "security"];

function AppShell() {
  const [view, setView] = useState<View>("home");
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus | null>(null);
  const [missions, setMissions] = useState<MissionSummary[]>([]);
  const [activeMission, setActiveMission] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [projectModal, setProjectModal] = useState<null | "open" | "new" | "choose">(null);
  const [projectAlert, setProjectAlert] = useState(false);
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
    // Restore the last-selected mission so that after a daemon restart the UI
    // reconnects to the same mission's authoritative state instead of showing
    // an empty shell.  A stale id simply resolves to daemon_unavailable /
    // no_mission through MissionView's real health check.
    const storedMission = window.localStorage.getItem("agentcode-active-mission");
    if (storedMission) setActiveMission(storedMission);
    // Use a ref for activeMission so the poll loop is created once and never
    // recreated when the active mission changes (single bounded interval).
    // Initialize it from the restored mission so the first poll does not
    // treat a persisted terminal mission as "nothing selected".
    const activeRef = { current: storedMission ?? activeMission };
    const refresh = async () => {
      const [d, m] = await Promise.all([daemon.health(), daemon.listActiveMissions()]);
      if (cancelled) return;
      setDaemonStatus(d);
      setMissions(m);
      // Select the first active mission only when nothing is selected yet.
      if (!activeRef.current && m.length > 0) setActiveMission(m[0].mission_id);
      // Intentionally DO NOT clear activeMission when it leaves the active
      // list: a terminal (completed/failed/cancelled) mission is still
      // retrievable from the daemon and must stay visible so the user sees
      // its real terminal state rather than a fabricated "no mission".
    };
    refresh();
    const timer = setInterval(refresh, 4000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, []);

  // Persist the last-selected mission id (never any secret/state) so a
  // daemon restart can reconnect to it.
  useEffect(() => {
    if (activeMission) {
      window.localStorage.setItem("agentcode-active-mission", activeMission);
    } else {
      window.localStorage.removeItem("agentcode-active-mission");
    }
  }, [activeMission]);

  const handleOpenedProject = (p: Project) => {
    openProject(p.path, p.name);
    setProjectModal(null);
    setProjectAlert(false);
    setView("home");
    setNotice(`Project "${p.name}" opened.`);
    setTimeout(() => setNotice(null), 4000);
  };

  const handleNewMission = async (
    goal: string
  ): Promise<{ ok: boolean; error?: string }> => {
    if (!project) {
      setProjectModal("choose");
      return { ok: false, error: "Open a project first." };
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
        return { ok: true };
      } else {
        // Surface the real backend error (e.g. a rejected workspace_root)
        // instead of a fabricated success or a generic failure message.
        const error = created.error || "Daemon is not connected — could not submit mission.";
        setNotice(error);
        setTimeout(() => setNotice(null), 6000);
        return { ok: false, error };
      }
    };
    return doSubmit();
  };

  const daemonConnected = daemonStatus?.state === "running";

  return (
    <div className="h-screen w-screen flex flex-col bg-background text-on-surface overflow-hidden font-body antialiased dark:bg-background dark:text-on-surface">
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          view={view}
          setView={requestView}
          daemonConnected={daemonConnected}
          recoveredSessions={daemonStatus?.recovered_sessions ?? 0}
          project={project}
          onOpenProject={() => setProjectModal(project ? "open" : "choose")}
        />
        <div className="flex-1 flex flex-col overflow-hidden">
          <TopBar
            project={project ? `Project: ${project.name}` : "No Project Open"}
            workspace={project?.path}
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
              onNavigate={(target) => {
                if (target === "mission") {
                  if (activeMission) setView("mission");
                  else setProjectAlert(true);
                } else {
                  setView(target);
                }
              }}
            />
          )}
          {view === "mission" && <MissionView missionId={activeMission} onOpenSettings={() => setView("settings")} />}
          {view === "chat" && (
            <ChatView
              project={project}
              onOpenMission={(missionId) => {
                setActiveMission(missionId);
                setView("mission");
              }}
              daemonConnected={daemonConnected}
            />
          )}
          {view === "discuss" && (
            <DiscussView
              project={project}
              daemonConnected={daemonConnected}
              onOpenMission={(missionId) => {
                setActiveMission(missionId);
                setView("mission");
              }}
            />
          )}
          {view === "design" && (
            <DesignView
              project={project}
              missionId={activeMission}
              daemonConnected={daemonConnected}
              onOpenMission={(missionId) => {
                setActiveMission(missionId);
                setView("mission");
              }}
            />
          )}
          {view === "security" && (
            <SecurityView
              project={project}
              daemonConnected={daemonConnected}
              onOpenMission={(missionId) => {
                setActiveMission(missionId);
                setView("mission");
              }}
              onOpenSettings={() => setView("settings")}
            />
          )}
          {view === "settings" && <SettingsView />}
          <Footer
            daemonConnected={daemonConnected}
            modelLabel="Model: daemon-managed"
            tasks={`Tasks: ${missions.length}`}
            recoveredSessions={daemonStatus?.recovered_sessions ?? 0}
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