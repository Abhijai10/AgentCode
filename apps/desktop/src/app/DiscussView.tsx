import { useEffect, useState } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { Project } from "./ProjectContext";
import type { MissionDetails, TaskDetail } from "./types";

function stateLabel(state: string): string {
  switch (state) {
    case "queued": return "Queued";
    case "pending": return "Pending";
    case "ready": return "Ready";
    case "running": return "Running";
    case "retryable": return "Retryable";
    case "completed": return "Completed";
    case "cancelled": return "Cancelled";
    case "paused": return "Paused";
    default:
      if (state.startsWith("failed:")) return "Failed";
      if (state === "failed") return "Failed";
      return state.charAt(0).toUpperCase() + state.slice(1);
  }
}

function stateColor(state: string): string {
  if (state === "completed") return "text-emerald-600 dark:text-emerald-400";
  if (state === "cancelled") return "text-on-surface-variant";
  if (state === "failed" || state.startsWith("failed:")) return "text-red-600 dark:text-red-400";
  if (state === "paused") return "text-amber-600 dark:text-amber-400";
  if (state === "running" || state === "retryable") return "text-primary";
  return "text-on-surface-variant";
}

export function DiscussView({
  project,
  missionId,
  onOpenMission,
  onNewMission,
}: {
  project: Project | null;
  missionId: string | null;
  onOpenMission(missionId: string): void;
  onNewMission(): void;
}) {
  const [details, setDetails] = useState<MissionDetails | null>(null);
  const [tasks, setTasks] = useState<TaskDetail[]>([]);
  const [missionStatus, setMissionStatus] = useState<"loading" | "no_mission" | "daemon_unavailable" | "loaded">(
    missionId ? "loading" : "no_mission"
  );

  useEffect(() => {
    if (!missionId) {
      setDetails(null);
      setTasks([]);
      setMissionStatus("no_mission");
      return;
    }
    let cancelled = false;
    setMissionStatus("loading");
    (async () => {
      const [d, t] = await Promise.all([
        daemon.getMissionDetails(missionId),
        daemon.getTaskDetails(missionId),
      ]);
      if (cancelled) return;
      if (d) {
        setDetails(d);
        setTasks(t ?? []);
        setMissionStatus("loaded");
      } else {
        const health = await daemon.health();
        if (cancelled) return;
        setMissionStatus(health.state === "running" ? "no_mission" : "daemon_unavailable");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [missionId]);

  return (
    <main className="flex-1 flex flex-col relative px-6 md:px-8 py-6 max-w-5xl mx-auto w-full overflow-hidden">
      <div className="flex-1 neo-raised p-6 flex flex-col mb-6 overflow-hidden min-h-0">
        <div className="flex items-center justify-between mb-6 pb-4 border-b border-outline-variant/40 dark:border-white/5">
          <div>
            <h1 className="text-2xl font-semibold text-on-surface">Discussion</h1>
            <p className="text-sm text-on-surface-variant mt-1">
              {project ? `Project: ${project.name}` : "No project open"}
            </p>
          </div>
          {missionId && (
            <button
              onClick={() => onOpenMission(missionId)}
              className="neo-button px-4 py-2 rounded-xl text-sm text-primary font-medium flex items-center gap-2"
            >
              <Icon name="terminal" size={16} />
              Open Mission
            </button>
          )}
        </div>

        {/* Mission context — real authoritative state from the daemon */}
        <div className="neo-pressed rounded-xl p-5 mb-4">
          <h2 className="text-sm font-bold text-on-surface-variant uppercase tracking-wider mb-3 flex items-center gap-2">
            <Icon name="terminal" size={16} className="text-primary" />
            Mission Context
          </h2>
          {missionStatus === "no_mission" && (
            <div className="text-sm text-on-surface-variant">
              No mission selected. Start a mission from Home to populate this workspace with real
              mission state.
            </div>
          )}
          {missionStatus === "daemon_unavailable" && (
            <div className="text-sm text-red-600 dark:text-red-400">
              The AgentCode daemon is not responding. Start it, then reopen this view to reload
              authoritative mission state.
            </div>
          )}
          {missionStatus === "loading" && (
            <div className="text-sm text-on-surface-variant flex items-center gap-2">
              <Icon name="autorenew" size={16} className="animate-spin" /> Loading mission…
            </div>
          )}
          {missionStatus === "loaded" && details && (
            <div className="space-y-3">
              <div className="flex items-center gap-3 flex-wrap">
                <span className={`px-2 py-0.5 rounded text-xs font-bold uppercase tracking-wider neo-pressed ${stateColor(details.state)}`}>
                  {stateLabel(details.state)}
                </span>
                {details.terminal && <span className="text-xs text-on-surface-variant">Final state</span>}
              </div>
              <p className="text-lg font-semibold text-on-surface leading-snug">{details.goal || "Mission"}</p>
              <div className="flex flex-wrap gap-x-6 gap-y-1 text-xs text-on-surface-variant">
                <span>{details.task_count} task{details.task_count !== 1 ? "s" : ""}</span>
                <span className="text-emerald-600 dark:text-emerald-400">{details.completed_task_count} completed</span>
                {details.failed_task_count > 0 && (
                  <span className="text-red-600 dark:text-red-400">{details.failed_task_count} failed</span>
                )}
                {details.current_task && details.state === "running" && (
                  <span className="flex items-center gap-1.5">
                    <Icon name="bolt" size={14} className="text-primary" />
                    Working on: {tasks.find((t) => t.task_id === details.current_task)?.title ?? details.current_task}
                  </span>
                )}
              </div>
              {tasks.length > 0 && (
                <ol className="space-y-1.5 pt-1">
                  {tasks.slice(0, 12).map((t) => {
                    const done = t.state === "completed" || t.state === "succeeded";
                    const inProgress = t.state === "running";
                    return (
                      <li key={t.task_id} className="flex items-center gap-2 text-sm">
                        {done ? (
                          <Icon name="check_circle" size={15} className="text-emerald-600 dark:text-emerald-400" fill />
                        ) : inProgress ? (
                          <Icon name="autorenew" size={15} className="text-primary animate-spin" />
                        ) : (
                          <Icon name="radio_button_unchecked" size={15} className="text-outline" />
                        )}
                        <span className={`flex-1 truncate ${done ? "line-through text-on-surface-variant" : "text-on-surface"}`}>
                          {t.title || t.task_id}
                        </span>
                        <span className={`text-[11px] font-bold uppercase tracking-wide ${stateColor(t.state)}`}>{stateLabel(t.state)}</span>
                      </li>
                    );
                  })}
                </ol>
              )}
              {tasks.length === 0 && (
                <p className="text-xs text-on-surface-variant">No plan has been recorded yet.</p>
              )}
            </div>
          )}
        </div>

        {/* Discussion capability — backend-bounded, never fabricated */}
        <div className="flex-1 overflow-y-auto pr-4">
          <div className="neo-pressed rounded-xl p-4 text-sm text-on-surface-variant flex items-start gap-2">
            <Icon name="info" size={16} className="text-primary mt-0.5" />
            <div>
              <p className="text-on-surface font-medium mb-1">Discussion capability</p>
              <p>
                The AgentCode daemon does not currently expose a persistent discussion/chat IPC
                contract, so a conversational agent is not available in this build. This workspace
                shows the real mission context above. Submit a coding mission from Home to do
                goal-driven work instead.
              </p>
            </div>
          </div>
        </div>

        <div className="mt-4 flex items-center justify-between gap-3">
          <button
            onClick={onNewMission}
            className="neo-button px-5 py-2.5 rounded-xl text-sm text-primary font-medium flex items-center gap-2"
          >
            <Icon name="add" size={16} />
            New Mission
          </button>
          <span className="text-xs text-on-surface-variant">
            Discussion requires a backend chat IPC — {missionId ? "mission context attached" : "no mission attached"}
          </span>
        </div>
      </div>
    </main>
  );
}
