import { useRef, useEffect, useState, useCallback, Component, type ReactNode } from "react";
import { ThemeProvider } from "./ThemeContext";
import { ProjectProvider, useProject, type Project } from "./ProjectContext";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import { Footer } from "./Footer";
import { HomeView } from "./HomeView";
// Final-audit optimization: code-split the heavy views — each is only
// loaded when its tab is first opened, so startup ships just the shell +
// Home.  Suspense fallback keeps the layout stable while a chunk loads.
import { lazy, Suspense } from "react";
const MissionView = lazy(() =>
  import("./MissionView").then((m) => ({ default: m.MissionView }))
);
const ChatView = lazy(() => import("./ChatView").then((m) => ({ default: m.ChatView })));
const DiscussView = lazy(() =>
  import("./DiscussView").then((m) => ({ default: m.DiscussView }))
);
const DesignView = lazy(() =>
  import("./DesignView").then((m) => ({ default: m.DesignView }))
);
const BrowserView = lazy(() =>
  import("./BrowserView").then((m) => ({ default: m.BrowserView }))
);
const SecurityView = lazy(() =>
  import("./SecurityView").then((m) => ({ default: m.SecurityView }))
);
const SettingsView = lazy(() =>
  import("./SettingsView").then((m) => ({ default: m.SettingsView }))
);
import { ProjectModal } from "./ProjectModal";
import { Icon } from "./Icon";
import { TerminalDrawer } from "./TerminalDrawer";
import { daemon } from "./daemon";
import type { View, MissionSummary, DaemonStatus } from "./types";

const PROJECT_REQUIRED_VIEWS: View[] = ["mission", "chat", "discuss", "design", "security"];
/// F5 (final audit): top-level error boundary — a thrown render error shows
/// a recovery screen with reload + error detail instead of white-screening
/// the whole window. State lives nowhere but this component; the daemon is
/// unaffected (the failure is renderer-side by construction).
class AppErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state: { error: Error | null } = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <div className="h-screen w-screen flex items-center justify-center bg-background p-8">
          <div className="neo-raised rounded-2xl p-6 max-w-lg text-center space-y-4">
            <Icon name="error" size={32} className="text-primary mx-auto" />
            <h2 className="text-lg font-semibold text-on-surface">Something broke in the interface</h2>
            <p className="text-xs text-on-surface-variant">
              The agent daemon is unaffected — this is a renderer error. Reload the
              window to reconnect; all state lives durably in the daemon.
            </p>
            <pre className="text-[10px] font-mono text-left text-on-surface-variant neo-pressed rounded-lg p-3 overflow-auto max-h-40">
              {String(this.state.error?.message ?? this.state.error)}
            </pre>
            <button
              className="neo-button px-4 py-2 rounded-lg text-sm text-primary font-medium"
              onClick={() => window.location.reload()}
            >
              Reload AgentCode
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

function AppShell() {
  const [view, setView] = useState<View>("home");
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus | null>(null);
  const [missions, setMissions] = useState<MissionSummary[]>([]);
  const missionStatesRef = useRef<Map<string, string>>(new Map());
  const [activeMission, setActiveMission] = useState<string | null>(null);
  // Watch-the-agent surfaces: inbuilt browser + terminal toggles live in
  // the sidebar rail (always reachable, like Codex).  The browser panel is
  // owned by DesignView; the terminal drawer by MissionView — the toggles
  // navigate to the owning view and open/close the surface.
  const [browserOpen, setBrowserOpen] = useState(false);
  const [terminalOpen, setTerminalOpen] = useState(false);
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
      // Leaving the browser view closes it — the sidebar rail icon must
      // always reflect the real state.
      if (target !== "browser") setBrowserOpen(false);
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
      // Work-completion notifications: a mission that was running last
      // tick and is now in a terminal state gets a toast.
      const TERMINAL = ["completed", "failed", "cancelled", "succeeded"];
      for (const mission of m) {
        const before = missionStatesRef.current.get(mission.mission_id);
        const now = mission.state;
        if (
          before && before !== now &&
          TERMINAL.some((t) => now.toLowerCase().includes(t)) &&
          !TERMINAL.some((t) => before.toLowerCase().includes(t))
        ) {
          const bad = now.toLowerCase().includes("fail") || now.toLowerCase().includes("cancel");
          setNotice(
            (bad ? "Mission ended: " : "Mission completed: ") +
            (mission.goal ? mission.goal.slice(0, 70) : mission.mission_id)
          );
          setTimeout(() => setNotice(null), 5000);
        }
        missionStatesRef.current.set(mission.mission_id, now);
      }
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
          browserOpen={browserOpen}
          onToggleBrowser={() => {
            // The browser is its own full view (Codex-IDE-style tab).  The
            // toggle always shows real state: on the browser view, tapping
            // again LEAVES it (back to home); from anywhere else it opens.
            if (view === "browser") {
              setBrowserOpen(false);
              setView("home");
            } else {
              setBrowserOpen(true);
              setView("browser");
            }
          }}
          terminalOpen={terminalOpen}
          onToggleTerminal={() => {
            setTerminalOpen((open) => !open);
          }}
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
              onOpenConversation={(mode) => {
                // The home composer started a conversation — land the user
                // in the matching view where the conversation continues.
                if (mode === "DISCUSS") setView("discuss");
                else if (mode === "DESIGN") setView("design");
                else if (mode === "SECURITY") setView("security");
                else setView("chat");
              }}
            />
          )}
          {view === "mission" && <Suspense fallback={<ViewLoading />}><MissionView missionId={activeMission} onOpenSettings={() => setView("settings")} /></Suspense>}
          {view === "chat" && (
            <Suspense fallback={<ViewLoading />}>
            <ChatView
              project={project}
              onOpenMission={(missionId) => {
                setActiveMission(missionId);
                setView("mission");
              }}
              daemonConnected={daemonConnected}
            />
            </Suspense>
          )}
          {view === "discuss" && (
            <Suspense fallback={<ViewLoading />}>
            <DiscussView
              project={project}
              daemonConnected={daemonConnected}
              onOpenMission={(missionId) => {
                setActiveMission(missionId);
                setView("mission");
              }}
            />
            </Suspense>
          )}
          {view === "design" && (
            <Suspense fallback={<ViewLoading />}>
            <DesignView
              project={project}
              missionId={activeMission}
              daemonConnected={daemonConnected}
              browserOpen={browserOpen}
              onBrowserOpenChange={setBrowserOpen}
              onOpenMission={(missionId) => {
                setActiveMission(missionId);
                setView("mission");
              }}
            />
            </Suspense>
          )}
          {view === "security" && (
            <Suspense fallback={<ViewLoading />}>
            <SecurityView
              project={project}
              daemonConnected={daemonConnected}
              onOpenMission={(missionId) => {
                setActiveMission(missionId);
                setView("mission");
              }}
              onOpenSettings={() => setView("settings")}
            />
            </Suspense>
          )}
          {view === "settings" && <Suspense fallback={<ViewLoading />}><SettingsView /></Suspense>}
          {view === "browser" && <Suspense fallback={<ViewLoading />}><BrowserView /></Suspense>}
          {/* Codex parity: the terminal is a GLOBAL bottom drawer available on
              every view — not a MissionView sub-panel. */}
          <TerminalDrawer
            open={terminalOpen}
            onOpenChange={setTerminalOpen}
            projectPath={project?.path ?? null}
          />
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

/// Code-split fallback: keeps the layout stable while a lazy view chunk
/// loads (same visual language as the loading states inside views).
function ViewLoading() {
  return (
    <div className="flex-1 flex items-center justify-center text-sm text-on-surface-variant">
      Loading…
    </div>
  );
}

export function App() {
  return (
    <ThemeProvider>
      <AppErrorBoundary>
        <ProjectProvider>
          <AppShell />
        </ProjectProvider>
      </AppErrorBoundary>
    </ThemeProvider>
  );
}