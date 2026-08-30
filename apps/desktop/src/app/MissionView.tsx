import { useEffect, useState } from "react";
import { Icon } from "./Icon";
import type { MissionProgress, ActivityItem, TaskState, ChangeSet } from "./types";
import { daemon } from "./daemon";

const CATEGORY_META: Record<ActivityItem["category"], { label: string; icon: string; color: string }> = {
  analysis: { label: "Analysis", icon: "manage_search", color: "text-primary" },
  implementation: { label: "Implementation", icon: "code", color: "text-primary" },
  testing: { label: "Testing", icon: "science", color: "text-tertiary" },
  verification: { label: "Verification", icon: "verified_user", color: "text-emerald-600 dark:text-emerald-400" },
  recovery: { label: "Recovery", icon: "autorenew", color: "text-amber-600 dark:text-amber-400" },
};

export function MissionView({ missionId, onOpenSettings }: { missionId: string | null; onOpenSettings(): void }) {
  const [progress, setProgress] = useState<MissionProgress | null>(null);
  const [tasks, setTasks] = useState<TaskState[]>([]);
  const [changeset, setChangeset] = useState<ChangeSet | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [status, setStatus] = useState<"loading" | "empty" | "loaded">(missionId ? "loading" : "empty");

  useEffect(() => {
    if (!missionId) {
      setStatus("empty");
      return;
    }
    let cancelled = false;
    const refresh = async () => {
      const [p, m] = await Promise.all([daemon.getMissionProgress(missionId), daemon.getMission(missionId)]);
      if (cancelled) return;
      setProgress(p);
      setTasks(m?.tasks ?? []);
      setStatus(p ? "loaded" : "empty");
      const cs = await daemon.getChangeset(missionId);
      if (!cancelled) setChangeset(cs);
    };
    refresh();
    const timer = setInterval(refresh, 3000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [missionId]);

  const toggleExpanded = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  if (status === "empty") {
    return (
      <main className="flex-1 overflow-y-auto p-8 flex items-center justify-center">
        <div className="text-center max-w-md space-y-4">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl neo-raised mx-auto text-primary">
            <Icon name="terminal" size={30} fill />
          </div>
          <h2 className="text-2xl font-semibold text-on-surface">No active mission</h2>
          <p className="text-on-surface-variant text-sm">Start a mission from Home, or open an existing one from the mission list.</p>
        </div>
      </main>
    );
  }

  if (status === "loading" || !progress) {
    return (
      <main className="flex-1 overflow-y-auto p-8 flex items-center justify-center">
        <p className="text-on-surface-variant">Loading mission...</p>
      </main>
    );
  }

  const doneTasks = tasks.filter((t) => t.state === "done" || t.state === "completed" || t.state === "succeeded").length;
  const progressPct = tasks.length ? Math.round((doneTasks / tasks.length) * 100) : progress.progress_pct;

  return (
    <main className="flex-1 overflow-y-auto p-6 md:p-8 relative flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-3 text-sm text-on-surface-variant">
          <span className="neo-pressed px-2 py-0.5 rounded text-xs font-bold text-primary uppercase tracking-wider">
            {progress.state}
          </span>
          <span>{progress.active_task ? `Working on: ${progress.active_task}` : "Mission running"}</span>
        </div>
        <h1 className="text-3xl md:text-4xl font-semibold text-on-surface tracking-tight">
          {progress.goal || "Active mission"}
        </h1>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 flex flex-col gap-6">
          {/* Agent thought process / plan */}
          <div className="neo-raised rounded-2xl p-6 flex flex-col gap-4">
            <div className="flex justify-between items-center mb-2">
              <h2 className="text-lg font-semibold flex items-center gap-2 text-on-surface">
                <Icon name="psychology" size={20} className="text-primary" />
                Agent Progress
              </h2>
              <span className="px-3 py-1 text-xs font-bold text-primary neo-pressed rounded-full flex items-center gap-1">
                <span className="w-2 h-2 rounded-full bg-primary animate-pulse" />
                {progress.state === "complete" ? "Complete" : "Working"}
              </span>
            </div>

            <div className="w-full bg-surface-container-lowest rounded-full h-2 neo-pressed overflow-hidden">
              <div
                className="h-full bg-primary rounded-full transition-all duration-700"
                style={{ width: `${progressPct}%` }}
              />
            </div>

            {tasks.length > 0 && (
              <ol className="space-y-3 pl-2">
                {tasks.map((t) => {
                  const done = t.state === "done" || t.state === "completed" || t.state === "succeeded";
                  const inProgress = t.state === "in_progress" || t.state === "running" || t.state === "working" || t.state === "verifying";
                  return (
                    <li key={t.task_id} className={`flex items-start gap-3 ${done ? "opacity-60" : ""}`}>
                      {done ? (
                        <Icon name="check_circle" size={20} className="text-primary" fill />
                      ) : inProgress ? (
                        <Icon name="autorenew" size={20} className="text-primary animate-spin" />
                      ) : (
                        <Icon name="radio_button_unchecked" size={20} className="text-outline" />
                      )}
                      <div>
                        <p className={`font-medium ${done ? "line-through text-on-surface" : inProgress ? "font-bold text-primary" : "text-on-surface"}`}>
                          {t.title || t.task_id}
                        </p>
                      </div>
                    </li>
                  );
                })}
              </ol>
            )}
          </div>

          {/* Grouped activity */}
          <div className="neo-raised rounded-2xl p-6 flex flex-col gap-2">
            <h2 className="text-lg font-semibold flex items-center gap-2 text-on-surface mb-2">
              <Icon name="timeline" size={20} className="text-primary" />
              Activity
            </h2>
            {progress.activity.length === 0 ? (
              <p className="text-sm text-on-surface-variant">No activity yet.</p>
            ) : (
              <div className="space-y-3">
                {progress.activity.map((item) => {
                  const meta = CATEGORY_META[item.category] ?? CATEGORY_META.analysis;
                  const isExpanded = expanded.has(item.id);
                  return (
                    <div key={item.id} className="rounded-xl neo-pressed p-4">
                      <button
                        onClick={() => item.raw_events?.length && toggleExpanded(item.id)}
                        className="w-full flex items-start gap-3 text-left"
                      >
                        <span className={`mt-0.5 ${meta.color}`}>
                          <Icon name={meta.icon} size={18} fill />
                        </span>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-baseline justify-between gap-2">
                            <p className="text-sm font-semibold text-on-surface">
                              <span className="text-on-surface-variant font-medium mr-2">{meta.label}</span>
                              {item.summary}
                            </p>
                            <span className="text-xs text-on-surface-variant whitespace-nowrap">{item.timestamp}</span>
                          </div>
                          {item.detail && !isExpanded && (
                            <p className="text-xs text-on-surface-variant mt-1 truncate">{item.detail}</p>
                          )}
                          {item.files_changed && item.files_changed.length > 0 && (
                            <div className="flex flex-wrap gap-1.5 mt-2">
                              {item.files_changed.map((f) => (
                                <span key={f} className="text-[11px] text-on-surface-variant neo-button px-2 py-0.5 rounded-md font-mono">
                                  {f}
                                </span>
                              ))}
                            </div>
                          )}
                          {item.tests && (
                            <div className="mt-2 text-xs">
                              <span className="text-emerald-600 dark:text-emerald-400 font-medium">{item.tests.passed} passed</span>
                              <span className="text-on-surface-variant"> / {item.tests.total} tests</span>
                            </div>
                          )}
                          {item.errors && item.errors.length > 0 && (
                            <div className="mt-2 space-y-1">
                              {item.errors.map((e, i) => (
                                <p key={i} className="text-xs text-red-600 dark:text-red-400 font-medium">
                                  <Icon name="error" size={14} className="inline mr-1" />
                                  {e}
                                </p>
                              ))}
                            </div>
                          )}
                        </div>
                        {item.raw_events?.length ? (
                          <Icon name={isExpanded ? "expand_less" : "expand_more"} size={18} className="text-on-surface-variant" />
                        ) : null}
                      </button>
                      {isExpanded && item.raw_events && (
                        <pre className="mt-3 text-[11px] font-code text-on-surface-variant bg-surface-container-lowest rounded-lg p-3 overflow-x-auto whitespace-pre-wrap">
                          {item.raw_events.join("\n")}
                        </pre>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* Right column */}
        <div className="flex flex-col gap-6">
          {/* Environment stats — only rendered when the daemon reports real data */}
          {progress.files_read !== undefined || progress.files_changed !== undefined ? (
            <div className="neo-raised rounded-2xl p-5 flex flex-col gap-4">
              <h3 className="text-sm font-bold text-on-surface-variant uppercase tracking-wider">Environment</h3>
              <div className="grid grid-cols-2 gap-4">
                {progress.files_read !== undefined && (
                  <div className="neo-pressed rounded-xl p-3 text-center">
                    <div className="text-2xl font-light text-primary">{progress.files_read}</div>
                    <div className="text-xs text-on-surface-variant font-medium mt-1">Files Read</div>
                  </div>
                )}
                {progress.files_changed !== undefined && (
                  <div className="neo-pressed rounded-xl p-3 text-center">
                    <div className="text-2xl font-light text-tertiary">{progress.files_changed}</div>
                    <div className="text-xs text-on-surface-variant font-medium mt-1">Files Edited</div>
                  </div>
                )}
              </div>
              {progress.errors !== undefined && progress.errors > 0 && (
                <div className="neo-pressed rounded-xl p-3 flex items-center justify-between">
                  <span className="text-sm font-medium">Errors</span>
                  <span className="text-sm text-red-600 dark:text-red-400 font-code">{progress.errors}</span>
                </div>
              )}
              {progress.approvals_required !== undefined && progress.approvals_required > 0 && (
                <div className="neo-pressed rounded-xl p-3 flex items-center justify-between">
                  <span className="text-sm font-medium">Awaiting approval</span>
                  <span className="text-sm text-amber-600 dark:text-amber-400 font-code">{progress.approvals_required}</span>
                </div>
              )}
            </div>
          ) : null}

          {changeset && changeset.available && changeset.files && changeset.files.length > 0 && (
            <div className="neo-raised rounded-2xl p-5 flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-bold text-on-surface-variant uppercase tracking-wider">Changes</h3>
                <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${changeset.safe ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" : "bg-amber-500/10 text-amber-600"}`}>
                  {changeset.safe ? "Safe" : "Review"}
                </span>
              </div>
              {changeset.files.map((f) => (
                <div key={f.path} className="flex items-center justify-between neo-pressed rounded-lg px-3 py-2">
                  <span className="text-xs font-mono text-on-surface truncate">{f.path}</span>
                  <span className="text-[11px] flex gap-2">
                    <span className="text-emerald-600 dark:text-emerald-400">+{f.additions}</span>
                    <span className="text-red-600 dark:text-red-400">-{f.deletions}</span>
                  </span>
                </div>
              ))}
            </div>
          )}

          <div className="neo-raised rounded-2xl p-5">
            <h3 className="text-sm font-bold text-on-surface-variant uppercase tracking-wider mb-3">Verification</h3>
            <div className="space-y-2 text-xs text-on-surface-variant">
              <div className="flex items-center gap-2">
                <Icon name="verified_user" size={16} className="text-emerald-600 dark:text-emerald-400" />
                {changeset?.verification_state || "Pending"}
              </div>
              <button onClick={onOpenSettings} className="text-primary font-medium hover:underline mt-2">
                Open Settings
              </button>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}