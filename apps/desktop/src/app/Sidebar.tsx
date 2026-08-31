import type { View } from "./types";
import { Icon } from "./Icon";
import type { Project } from "./ProjectContext";

const NAV: { view: View; label: string; icon: string }[] = [
  { view: "home", label: "Home", icon: "home" },
  { view: "mission", label: "Mission", icon: "terminal" },
  { view: "discuss", label: "Discuss", icon: "forum" },
  { view: "design", label: "Design", icon: "draw" },
  { view: "security", label: "Security", icon: "security" },
];

export function Sidebar({
  view,
  setView,
  daemonConnected,
  recoveredSessions,
  project,
  onOpenProject,
}: {
  view: View;
  setView(v: View): void;
  daemonConnected: boolean;
  recoveredSessions: number;
  project: Project | null;
  onOpenProject(): void;
}) {
  return (
    <aside className="w-64 h-full shrink-0 flex flex-col py-6 bg-surface-container-low border-r border-outline-variant/40 dark:bg-surface-container-low dark:border-white/5">
      <div className="px-6 mb-8 flex items-center gap-3">
        <div className="w-10 h-10 rounded-xl neo-raised flex items-center justify-center text-primary">
          <Icon name="terminal" size={22} fill />
        </div>
        <div>
          <h1 className="font-bold text-on-surface tracking-tight text-lg leading-tight">AgentCode</h1>
          <p className="text-xs text-on-surface-variant">v1.0.4</p>
        </div>
      </div>

      <div className="px-4 mb-6 space-y-2">
        {project && (
          <div className="neo-pressed rounded-xl p-3 flex items-center gap-2 mb-3">
            <Icon name="folder" size={16} className="text-primary shrink-0" />
            <div className="min-w-0 flex-1">
              <p className="text-xs font-semibold text-on-surface truncate">{project.name}</p>
              <p className="text-[10px] text-on-surface-variant truncate">{project.path}</p>
            </div>
          </div>
        )}
        <button
          onClick={onOpenProject}
          className="w-full neo-button rounded-xl py-3 px-4 flex items-center justify-center gap-2 font-medium text-primary transition-all duration-150 active:scale-[0.98]"
        >
          <Icon name="folder_open" size={18} />
          {project ? "Switch Project" : "Open Project"}
        </button>
      </div>

      <nav className="flex-1 px-4 space-y-2 overflow-y-auto">
        {NAV.map((item) => (
          <button
            key={item.view}
            onClick={() => setView(item.view)}
            className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-colors duration-150 text-left ${
              view === item.view
                ? "text-primary font-bold bg-primary/5 neo-pressed"
                : "text-on-surface-variant hover:text-on-surface hover:bg-surface-variant/40 dark:hover:bg-white/5"
            }`}
          >
            <Icon name={item.icon} fill={view === item.view} />
            <span className="text-sm">{item.label}</span>
          </button>
        ))}
      </nav>

      <div className="px-4 mt-auto">
        <button
          onClick={() => setView("settings")}
          className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-colors duration-150 text-left ${
            view === "settings"
              ? "text-primary font-bold bg-primary/5 neo-pressed"
              : "text-on-surface-variant hover:text-on-surface hover:bg-surface-variant/40 dark:hover:bg-white/5"
          }`}
        >
          <Icon name="settings" fill={view === "settings"} />
          <span className="text-sm">Settings</span>
        </button>
        <div className="mt-3 mx-3 py-3 px-3 rounded-xl flex items-center gap-2 text-xs text-on-surface-variant">
          <span className={`w-2 h-2 rounded-full ${daemonConnected ? "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]" : "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.5)]"}`} />
          {daemonConnected ? "Daemon Connected" : "Daemon Disconnected"}
        </div>
        {daemonConnected && recoveredSessions > 0 && (
          <div className="mx-3 mb-3 py-2 px-3 rounded-xl flex items-center gap-2 text-xs text-amber-600 dark:text-amber-400">
            <Icon name="restart_alt" size={14} />
            Recovered {recoveredSessions} interrupted session{recoveredSessions !== 1 ? "s" : ""} from last run
          </div>
        )}
      </div>
    </aside>
  );
}