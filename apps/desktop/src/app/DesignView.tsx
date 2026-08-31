import { useEffect, useState } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { Project } from "./ProjectContext";
import type {
  MissionDetails,
  TaskDetail,
  ChangeSetSummary,
  VerificationSummary,
} from "./types";

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

export function DesignView({
  project,
  missionId,
  onOpenMission,
}: {
  project: Project | null;
  missionId: string | null;
  onOpenMission(missionId: string): void;
}) {
  const [details, setDetails] = useState<MissionDetails | null>(null);
  const [tasks, setTasks] = useState<TaskDetail[]>([]);
  const [changesets, setChangesets] = useState<ChangeSetSummary[]>([]);
  const [verification, setVerification] = useState<VerificationSummary | null>(null);
  const [status, setStatus] = useState<"loading" | "no_mission" | "daemon_unavailable" | "loaded">(
    missionId ? "loading" : "no_mission"
  );

  useEffect(() => {
    if (!missionId) {
      setDetails(null);
      setTasks([]);
      setChangesets([]);
      setVerification(null);
      setStatus("no_mission");
      return;
    }
    let cancelled = false;
    setStatus("loading");
    (async () => {
      const [d, t, c, v] = await Promise.all([
        daemon.getMissionDetails(missionId),
        daemon.getTaskDetails(missionId),
        daemon.getChangeSetSummary(missionId),
        daemon.getVerificationSummary(missionId),
      ]);
      if (cancelled) return;
      if (d) {
        setDetails(d);
        setTasks(t ?? []);
        setChangesets(c ?? []);
        setVerification(v);
        setStatus("loaded");
      } else {
        const health = await daemon.health();
        if (cancelled) return;
        setStatus(health.state === "running" ? "no_mission" : "daemon_unavailable");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [missionId]);

  return (
    <main className="flex-1 flex flex-col overflow-hidden">
      <div className="flex-1 overflow-y-auto p-6 md:p-8">
        <div className="max-w-5xl mx-auto flex flex-col gap-6">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h2 className="text-2xl font-semibold text-on-surface tracking-tight">Design &amp; Plan</h2>
              <p className="text-sm text-on-surface-variant mt-1">
                {project ? `Project: ${project.name}` : "No project open"} — real task plan and
                architecture state from the daemon.
              </p>
            </div>
            {missionId && (
              <button
                onClick={() => onOpenMission(missionId)}
                className="neo-button px-4 py-2 rounded-xl text-sm text-primary font-medium flex items-center gap-2 shrink-0"
              >
                <Icon name="terminal" size={16} />
                Open Mission
              </button>
            )}
          </div>

          {status === "no_mission" && (
            <div className="neo-raised rounded-2xl p-8 flex flex-col items-center text-center">
              <div className="w-14 h-14 rounded-2xl neo-raised mb-4 flex items-center justify-center text-primary">
                <Icon name="design_services" size={28} />
              </div>
              <h3 className="text-xl font-semibold text-on-surface mb-2">No mission selected</h3>
              <p className="text-sm text-on-surface-variant max-w-md mx-auto">
                Start a mission from Home to populate this view with the real task plan,
                dependencies, and verification state.
              </p>
            </div>
          )}

          {status === "daemon_unavailable" && (
            <div className="neo-raised rounded-2xl p-8 flex flex-col items-center text-center">
              <div className="w-14 h-14 rounded-2xl neo-raised mb-4 flex items-center justify-center text-red-600 dark:text-red-400">
                <Icon name="dns_off" size={28} />
              </div>
              <h3 className="text-xl font-semibold text-on-surface mb-2">Daemon unavailable</h3>
              <p className="text-sm text-on-surface-variant max-w-md mx-auto">
                The AgentCode daemon is not responding. Start it, then reopen this view to reload
                authoritative mission state.
              </p>
            </div>
          )}

          {status === "loading" && (
            <div className="neo-raised rounded-2xl p-8 flex items-center justify-center text-on-surface-variant gap-2">
              <Icon name="autorenew" size={18} className="animate-spin" />
              Loading plan…
            </div>
          )}

          {status === "loaded" && details && (
            <>
              <div className="neo-raised rounded-2xl p-6 flex flex-col gap-4">
                <div className="flex items-center gap-3 flex-wrap">
                  <span className={`px-2 py-0.5 rounded text-xs font-bold uppercase tracking-wider neo-pressed ${stateColor(details.state)}`}>
                    {stateLabel(details.state)}
                  </span>
                  {details.terminal && <span className="text-xs text-on-surface-variant">Final state</span>}
                </div>
                <h3 className="text-xl font-semibold text-on-surface">{details.goal || "Mission"}</h3>
                {details.workspace_root && (
                  <p className="text-xs text-on-surface-variant font-mono truncate">{details.workspace_root}</p>
                )}
                <div className="flex flex-wrap gap-x-6 gap-y-1 text-xs text-on-surface-variant">
                  <span>{details.task_count} task{details.task_count !== 1 ? "s" : ""}</span>
                  <span className="text-emerald-600 dark:text-emerald-400">{details.completed_task_count} completed</span>
                  {details.failed_task_count > 0 && (
                    <span className="text-red-600 dark:text-red-400">{details.failed_task_count} failed</span>
                  )}
                </div>
              </div>

              <div className="neo-raised rounded-2xl p-6 flex flex-col gap-3">
                <h3 className="text-lg font-semibold text-on-surface mb-1 flex items-center gap-2">
                  <Icon name="account_tree" size={20} className="text-primary" />
                  Task Plan &amp; Dependencies
                </h3>
                {tasks.length === 0 ? (
                  <p className="text-sm text-on-surface-variant">No plan has been recorded yet.</p>
                ) : (
                  <ol className="space-y-2">
                    {tasks.map((t) => {
                      const done = t.state === "completed" || t.state === "succeeded";
                      const inProgress = t.state === "running";
                      const failed = t.state === "failed";
                      return (
                        <li key={t.task_id} className={`rounded-xl neo-pressed p-3 ${done ? "opacity-70" : ""}`}>
                          <div className="flex items-start gap-3">
                            {done ? (
                              <Icon name="check_circle" size={20} className="text-emerald-600 dark:text-emerald-400 mt-0.5" fill />
                            ) : inProgress ? (
                              <Icon name="autorenew" size={20} className="text-primary mt-0.5 animate-spin" />
                            ) : failed ? (
                              <Icon name="cancel" size={20} className="text-red-600 dark:text-red-400 mt-0.5" />
                            ) : (
                              <Icon name="radio_button_unchecked" size={20} className="text-outline mt-0.5" />
                            )}
                            <div className="flex-1 min-w-0">
                              <div className="flex items-baseline justify-between gap-2 flex-wrap">
                                <p className={`font-medium ${done ? "line-through text-on-surface" : inProgress ? "font-bold text-primary" : "text-on-surface"}`}>
                                  {t.title || t.task_id}
                                </p>
                                <span className={`text-[11px] font-bold uppercase tracking-wide ${stateColor(t.state)}`}>{stateLabel(t.state)}</span>
                              </div>
                              {t.dependencies.length > 0 && (
                                <p className="text-[11px] text-on-surface-variant mt-0.5">
                                  Depends on {t.dependencies.length} task{t.dependencies.length !== 1 ? "s" : ""}
                                </p>
                              )}
                              {t.retry_count > 0 && (
                                <p className="text-[11px] text-amber-600 dark:text-amber-400 mt-0.5">
                                  Retried {t.retry_count}× (max {t.max_retries})
                                </p>
                              )}
                            </div>
                          </div>
                        </li>
                      );
                    })}
                  </ol>
                )}
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div className="neo-raised rounded-2xl p-5 flex flex-col gap-3">
                  <h4 className="text-sm font-bold text-on-surface-variant uppercase tracking-wider">Verification</h4>
                  {verification && verification.verifications.length > 0 ? (
                    <div className="space-y-2">
                      {verification.verifications.map((run) => (
                        <div key={run.verification_id} className="neo-pressed rounded-xl p-3">
                          <div className="flex items-center justify-between gap-2">
                            <span className="flex items-center gap-1.5 text-sm font-semibold text-on-surface">
                              <Icon name={run.passed ? "verified_user" : "report_problem"} size={16} className={run.passed ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"} />
                              {run.passed ? "Passed" : run.status}
                            </span>
                          </div>
                          <p className="text-[11px] text-on-surface-variant mt-1 font-mono truncate">{run.command}</p>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="text-xs text-on-surface-variant">No verification runs recorded yet.</p>
                  )}
                  {verification && verification.final_audits.length > 0 && (
                    <div className="neo-pressed rounded-xl p-3 flex items-center justify-between">
                      <span className="text-xs text-on-surface">Completion gate</span>
                      <span className={`text-xs font-bold ${verification.final_audits[0].passed ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}>
                        {verification.final_audits[0].passed ? "Passed" : "Not passed"}
                      </span>
                    </div>
                  )}
                </div>

                <div className="neo-raised rounded-2xl p-5 flex flex-col gap-3">
                  <h4 className="text-sm font-bold text-on-surface-variant uppercase tracking-wider">Changes</h4>
                  {changesets.length === 0 ? (
                    <p className="text-xs text-on-surface-variant">No ChangeSet recorded yet.</p>
                  ) : (
                    <div className="space-y-2">
                      {changesets.map((cs) => (
                        <div key={cs.changeset_id} className="neo-pressed rounded-xl p-3">
                          <div className="flex items-center justify-between gap-2">
                            <span className={`text-[11px] font-bold uppercase tracking-wide ${cs.applied ? "text-emerald-600 dark:text-emerald-400" : "text-amber-600 dark:text-amber-400"}`}>
                              {cs.state}
                            </span>
                          </div>
                          {cs.files.map((f) => (
                            <div key={f.path} className="flex items-center justify-between gap-2 mt-1.5">
                              <span className="text-[11px] font-mono text-on-surface truncate">{f.path}</span>
                              <span className="text-[10px] flex gap-1.5 shrink-0">
                                <span className="text-emerald-600 dark:text-emerald-400">+{f.additions}</span>
                                <span className="text-red-600 dark:text-red-400">-{f.removals}</span>
                              </span>
                            </div>
                          ))}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </>
          )}

          <div className="neo-pressed rounded-xl p-4 text-sm text-on-surface-variant flex items-start gap-2">
            <Icon name="info" size={16} className="text-primary mt-0.5" />
            <div>
              <p className="text-on-surface font-medium mb-1">Design Studio sessions</p>
              <p>
                Persistent Design Studio session artifacts are not exposed by the daemon IPC in this
                build. This view shows the real task plan, dependencies, verification, and change
                state for the current mission instead of fabricated design documents.
              </p>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
