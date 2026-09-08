import { useCallback, useEffect, useRef, useState } from "react";
import { Icon } from "./Icon";
import type {
  MissionDetails,
  TaskDetail,
  MissionActivityEvent,
  ChangeSetSummary,
  EvidenceSummaryItem,
  VerificationSummary,
} from "./types";
import { daemon } from "./daemon";

const POLL_INTERVAL_MS = 2500;
const MAX_RENDERED_EVENTS = 60;

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

function fmtTime(ms: number | undefined): string {
  if (ms === undefined || ms === null || ms <= 0) return "—";
  const d = new Date(ms);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function fmtElapsed(ms: number | undefined): string {
  if (ms === undefined || ms === null || ms <= 0) return "—";
  const secs = Math.max(0, Math.floor((Date.now() - ms) / 1000));
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

const EVENT_LABEL: Record<string, { label: string; icon: string; cls: string }> = {
  "kernel.create_mission": { label: "Mission created", icon: "add_circle", cls: "text-primary" },
  "kernel.activate_mission": { label: "Mission started", icon: "play_arrow", cls: "text-primary" },
  "kernel.complete_mission": { label: "Mission completed", icon: "check_circle", cls: "text-emerald-600 dark:text-emerald-400" },
  "kernel.cancel_mission": { label: "Mission cancelled", icon: "cancel", cls: "text-on-surface-variant" },
  "kernel.approve_changeset": { label: "ChangeSet approved", icon: "verified_user", cls: "text-primary" },
};

function eventMeta(kind: string): { label: string; icon: string; cls: string } {
  const direct = EVENT_LABEL[kind];
  if (direct) return direct;
  if (kind.startsWith("attempt.")) {
    const outcome = kind.replace("attempt.", "");
    return {
      label: outcome === "succeeded" ? "Agent attempt succeeded" : `Agent attempt ${outcome}`,
      icon: outcome === "succeeded" ? "task_alt" : "sync_problem",
      cls: outcome === "succeeded" ? "text-emerald-600 dark:text-emerald-400" : "text-amber-600 dark:text-amber-400",
    };
  }
  if (kind.startsWith("verification.")) {
    const status = kind.replace("verification.", "");
    const ok = status === "Passed";
    return {
      label: ok ? "Verification passed" : `Verification ${status}`,
      icon: ok ? "verified_user" : "report_problem",
      cls: ok ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400",
    };
  }
  if (kind.startsWith("changeset.")) {
    const state = kind.replace("changeset.", "");
    return {
      label: `ChangeSet ${state.toLowerCase()}`,
      icon: "difference",
      cls: state === "Applied" || state === "Accepted" ? "text-emerald-600 dark:text-emerald-400" : "text-primary",
    };
  }
  if (kind.startsWith("evidence.")) {
    return { label: "Evidence recorded", icon: "inventory_2", cls: "text-on-surface-variant" };
  }
  return { label: kind.replace(/_/g, " "), icon: "info", cls: "text-on-surface-variant" };
}

export function MissionView({ missionId, onOpenSettings }: { missionId: string | null; onOpenSettings(): void }) {
  const [details, setDetails] = useState<MissionDetails | null>(null);
  const [tasks, setTasks] = useState<TaskDetail[]>([]);
  const [events, setEvents] = useState<MissionActivityEvent[]>([]);
  const [changesets, setChangesets] = useState<ChangeSetSummary[]>([]);
  const [evidence, setEvidence] = useState<EvidenceSummaryItem[]>([]);
  // Honesty inspector: the full task→attempt→evidence→audit chain.
  const [whyOpen, setWhyOpen] = useState(false);
  const [whyChain, setWhyChain] = useState<Awaited<ReturnType<typeof daemon.evidenceChain>> | null>(null);
  const [whyBusy, setWhyBusy] = useState(false);
  const [verification, setVerification] = useState<VerificationSummary | null>(null);
  const [status, setStatus] = useState<"loading" | "no_mission" | "daemon_unavailable" | "loaded">(missionId ? "loading" : "no_mission");
  const [controlBusy, setControlBusy] = useState<"pause" | "resume" | "cancel" | null>(null);
  const [controlError, setControlError] = useState<string | null>(null);
  // Batch N6: export the mission result context (governed write path).
  const [exportResult, setExportResult] = useState<{ path: string } | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exportBusy, setExportBusy] = useState(false);



  const handleExport = () =>
    void (async () => {
      if (!missionId) return;
      setExportBusy(true);
      setExportError(null);
      try {
        const res = await daemon.missionExport(missionId);
        if (res.ok && res.export) {
          const doc = res.export as { path?: string };
          setExportResult({ path: doc.path ?? "" });
        } else {
          setExportError(res.error ?? "export failed");
        }
      } finally {
        setExportBusy(false);
      }
    })();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const timerRef = useRef<number | null>(null);

  const stopPolling = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }
  }, []);
  // Event-burst refresh coalescing (component-scope refs — hooks inside
  // an effect crash production builds with React #321):
  const lastRefreshRef = useRef(0);
  const pendingRefreshRef = useRef<number | null>(null);
  const cancelledRef = useRef(false);

  useEffect(() => {
    if (!missionId) {
      setStatus("no_mission");
      stopPolling();
      return;
    }
    cancelledRef.current = false;
    let sawTerminal = false;
    setStatus("loading");
    setControlError(null);

    const refresh = async () => {
      if (!cancelledRef.current) {
        const now = Date.now();
        const since = now - lastRefreshRef.current;
        if (since < 750) {
          if (pendingRefreshRef.current === null) {
            pendingRefreshRef.current = window.setTimeout(() => {
              pendingRefreshRef.current = null;
              void refresh();
            }, 750 - since);
          }
          return;
        }
        lastRefreshRef.current = now;
      }
      const [d, t, e, c, ev, v] = await Promise.all([
        daemon.getMissionDetails(missionId),
        daemon.getTaskDetails(missionId),
        daemon.getMissionEvents(missionId, MAX_RENDERED_EVENTS),
        daemon.getChangeSetSummary(missionId),
        daemon.getEvidenceSummary(missionId),
        daemon.getVerificationSummary(missionId),
      ]);
      if (cancelledRef.current) return;
      if (d) {
        setDetails(d);
        setStatus("loaded");
      } else {
        // A stopped daemon must show "Daemon unavailable" rather than a
        // misleading "No mission selected" — ask the real health signal.
        const health = await daemon.health();
        if (cancelledRef.current) return;
        setStatus(health.state === "running" ? "no_mission" : "daemon_unavailable");
      }
      if (t) setTasks(t);
      if (e) setEvents(e);
      if (c) setChangesets(c);
      if (ev) setEvidence(ev);
      if (v) setVerification(v);
      // Terminal mission: stop polling, keep the final snapshot rendered.
      if (d?.terminal) {
        sawTerminal = true;
        stopPolling();
      }
    };

  refresh();
  stopPolling();
  // Doc 06 H4 event-push: while this mission is live, EventsSubscribe
  // long-polls the kernel event stream — each returned batch triggers one
  // heavy refresh.  Falls back to the interval poll when the stream is
  // unavailable; stops on terminal state exactly as before.
  const subscribeLoop = async (cursorMs: number, cursorId: string) => {
    if (cancelledRef.current) return;
    const res = await daemon.eventsSubscribe(cursorMs, cursorId, 5000);
    if (cancelledRef.current) return;
    if (!res) {
      timerRef.current = window.setInterval(refresh, POLL_INTERVAL_MS);
      return;
    }
    if (res.events.length > 0) {
      await refresh();
      if (cancelledRef.current || sawTerminal) return;
    }
    timerRef.current = window.setTimeout(
      () => void subscribeLoop(res.cursor.created_at_ms, res.cursor.id),
      50
    );
  };
  void subscribeLoop(-1, "");
  return () => {
    cancelledRef.current = true;
    stopPolling();
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

  const runControl = async (action: "pause" | "resume" | "cancel") => {
    if (!missionId || controlBusy) return;
    setControlBusy(action);
    setControlError(null);
    const result =
      action === "pause"
        ? await daemon.pauseMission(missionId)
        : action === "resume"
          ? await daemon.resumeMission(missionId)
          : await daemon.cancelMission(missionId);
    if (!result.ok) {
      // Surface the real backend error (e.g. an already-terminal mission)
      // instead of a generic message.
      setControlError(result.error || `The daemon rejected ${action}.`);
    }
    setControlBusy(null);
    // Refresh authoritative state immediately after the backend responds.
    const [d, t, e, c, ev, v] = await Promise.all([
      daemon.getMissionDetails(missionId),
      daemon.getTaskDetails(missionId),
      daemon.getMissionEvents(missionId, MAX_RENDERED_EVENTS),
      daemon.getChangeSetSummary(missionId),
      daemon.getEvidenceSummary(missionId),
      daemon.getVerificationSummary(missionId),
    ]);
    if (d) { setDetails(d); setStatus("loaded"); }
    if (t) setTasks(t);
    if (e) setEvents(e);
    if (c) setChangesets(c);
    if (ev) setEvidence(ev);
    if (v) setVerification(v);
  };


  if (status === "no_mission") {
    return (
      <div className="flex-1 flex flex-col overflow-hidden">
        <main className="flex-1 overflow-y-auto p-8 flex items-center justify-center">
          <div className="text-center max-w-md space-y-4">
            <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl neo-raised mx-auto text-primary">
              <Icon name="terminal" size={30} fill />
            </div>
            <h2 className="text-2xl font-semibold text-on-surface">No mission selected</h2>
            <p className="text-on-surface-variant text-sm">Start a mission from Home, or select one from the mission list.</p>
          </div>
        </main>
      </div>
    );
  }

  if (status === "daemon_unavailable") {
    return (
      <div className="flex-1 flex flex-col overflow-hidden">
        <main className="flex-1 overflow-y-auto p-8 flex items-center justify-center">
          <div className="text-center max-w-md space-y-4">
            <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl neo-raised mx-auto text-red-600 dark:text-red-400">
              <Icon name="dns_off" size={30} />
            </div>
            <h2 className="text-2xl font-semibold text-on-surface">Daemon unavailable</h2>
            <p className="text-on-surface-variant text-sm">The AgentCode daemon is not responding. Start it, then reopen this view to reload authoritative mission state.</p>
          </div>
        </main>
      </div>
    );
  }

  if (status === "loading" || !details) {
    return (
      <div className="flex-1 flex flex-col overflow-hidden">
        <main className="flex-1 overflow-y-auto p-8 flex items-center justify-center">
          <p className="text-on-surface-variant flex items-center gap-2">
            <Icon name="autorenew" size={18} className="animate-spin" />
            Loading mission…
          </p>
        </main>
      </div>
    );
  }

  const state = details.state;
  const terminal = details.terminal;
  const progressPct = details.task_count > 0 ? Math.round((details.progress ?? 0) * 100) : 0;
  // Pause is available for any non-terminal mission that is not already
  // paused.  The backend supports pausing queued and running missions alike
  // (P1 added active-mission pause), so we must not restrict Pause to
  // queued-only.
  const pausable = !terminal && !["paused", "cancelled", "failed", "completed"].includes(state) && !state.startsWith("failed:");
  const showPause = pausable;
  const showResume = state === "paused";
  const showCancel = !terminal && (state === "queued" || state === "paused" || state === "running" || state === "ready" || state === "retryable" || state === "pending");

  const renderedEvents = events.slice(0, MAX_RENDERED_EVENTS);

  return (
    <main className="flex-1 overflow-y-auto p-6 md:p-8 relative flex flex-col gap-6">
      {/* Header: goal + state */}
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-3 text-sm text-on-surface-variant flex-wrap">
          <span className={`neo-pressed px-2 py-0.5 rounded text-xs font-bold uppercase tracking-wider ${stateColor(state)}`}>
            {stateLabel(state)}
          </span>
          {details.current_task && state === "running" && (
            <span className="flex items-center gap-1.5">
              <Icon name="bolt" size={14} className="text-primary" />
              Working on: {tasks.find((t) => t.task_id === details.current_task)?.title ?? details.current_task}
            </span>
          )}
          {terminal && <span className="text-xs text-on-surface-variant">Final state</span>}
          {details.failure_code && (
            <span className="text-xs text-red-600 dark:text-red-400 font-code flex items-center gap-1">
              <Icon name="error" size={14} fill />
              {details.failure_code}
            </span>
          )}
        </div>
        <h1 className="text-3xl md:text-4xl font-semibold text-on-surface tracking-tight">
          {details.goal || "Mission"}
        </h1>
        <div className="flex flex-wrap gap-x-6 gap-y-1 text-xs text-on-surface-variant">
          {details.workspace_root && <span className="font-mono truncate max-w-[42ch]">{details.workspace_root}</span>}
          <span>Updated {fmtElapsed(details.updated_at_ms)} ago</span>
          <span>Created {fmtTime(details.created_at_ms)}</span>
        </div>
      </div>

      {/* Controls */}
      {(showPause || showResume || showCancel) && (
        <div className="flex flex-wrap gap-3">
          {showPause && (
            <button
              onClick={() => runControl("pause")}
              disabled={controlBusy !== null}
              className="neo-button px-5 py-2.5 rounded-xl text-sm font-medium text-amber-600 dark:text-amber-400 flex items-center gap-2 disabled:opacity-50"
            >
              <Icon name={controlBusy === "pause" ? "autorenew" : "pause"} size={18} className={controlBusy === "pause" ? "animate-spin" : ""} />
              Pause
            </button>
          )}
          {showResume && (
            <button
              onClick={() => runControl("resume")}
              disabled={controlBusy !== null}
              className="neo-button px-5 py-2.5 rounded-xl text-sm font-medium text-primary flex items-center gap-2 disabled:opacity-50"
            >
              <Icon name={controlBusy === "resume" ? "autorenew" : "play_arrow"} size={18} className={controlBusy === "resume" ? "animate-spin" : ""} />
              Resume
            </button>
          )}
          {showCancel && (
            <button
              onClick={() => runControl("cancel")}
              disabled={controlBusy !== null}
              className="neo-button px-5 py-2.5 rounded-xl text-sm font-medium text-red-600 dark:text-red-400 flex items-center gap-2 disabled:opacity-50"
            >
              <Icon name={controlBusy === "cancel" ? "autorenew" : "stop_circle"} size={18} className={controlBusy === "cancel" ? "animate-spin" : ""} />
              Cancel
            </button>
          )}
          <button
            onClick={handleExport}
            disabled={exportBusy}
            className="neo-button px-5 py-2.5 rounded-xl text-sm font-medium text-primary flex items-center gap-2 disabled:opacity-50"
          >
            <Icon name="ios_share" size={18} className={exportBusy ? "animate-spin" : ""} />
            Export Result
          </button>
        </div>
      )}
      {exportResult && (
        <p className="text-xs text-emerald-600 flex items-center gap-1.5 mt-1">
          <Icon name="verified_user" size={14} /> Exported to {exportResult.path}
        </p>
      )}
      {exportError && (
        <p className="text-xs text-red-600 flex items-center gap-1.5 mt-1">
          <Icon name="error" size={14} fill /> {exportError}
        </p>
      )}
      {controlError && (
        <div className="flex items-center gap-2 text-xs text-red-600 dark:text-red-400 font-medium">
          <Icon name="error" size={14} fill />
          {controlError}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 flex flex-col gap-6">
          {/* Overall progress */}
          <div className="neo-raised rounded-2xl p-6 flex flex-col gap-4">
            <div className="flex justify-between items-center mb-1">
              <h2 className="text-lg font-semibold flex items-center gap-2 text-on-surface">
                <Icon name="insights" size={20} className="text-primary" />
                Progress
              </h2>
              <span className="px-3 py-1 text-xs font-bold text-primary neo-pressed rounded-full">
                {progressPct}%
              </span>
            </div>
            <div className="w-full bg-surface-container-lowest rounded-full h-2 neo-pressed overflow-hidden">
              <div
                className="h-full bg-primary rounded-full transition-all duration-700"
                style={{ width: `${progressPct}%` }}
              />
            </div>
            <div className="flex flex-wrap gap-x-6 gap-y-1 text-xs text-on-surface-variant">
              <span>{details.task_count} task{details.task_count !== 1 ? "s" : ""}</span>
              <span className="text-emerald-600 dark:text-emerald-400">{details.completed_task_count} completed</span>
              {details.failed_task_count > 0 && (
                <span className="text-red-600 dark:text-red-400">{details.failed_task_count} failed</span>
              )}
              {details.completion && (
                <span className={details.completion.passed ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}>
                  {details.completion.completion_allowed ? "Completion gate passed" : "Completion gate not cleared"}
                </span>
              )}
            </div>
          </div>

          {/* Plan / tasks */}
          <div className="neo-raised rounded-2xl p-6 flex flex-col gap-3">
            <h2 className="text-lg font-semibold flex items-center gap-2 text-on-surface mb-1">
              <Icon name="account_tree" size={20} className="text-primary" />
              Plan
            </h2>
            {tasks.length === 0 ? (
              <p className="text-sm text-on-surface-variant">No plan has been recorded yet.</p>
            ) : (
              <ol className="space-y-2 pl-1">
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
                          {t.attempts.length > 0 && (
                            <button
                              onClick={() => toggleExpanded(t.task_id)}
                              className="mt-1.5 text-[11px] text-primary font-medium flex items-center gap-1"
                            >
                              {t.attempts.length} attempt{t.attempts.length !== 1 ? "s" : ""}
                              <Icon name={expanded.has(t.task_id) ? "expand_less" : "expand_more"} size={14} />
                            </button>
                          )}
                          {expanded.has(t.task_id) && (
                            <div className="mt-2 space-y-1.5">
                              {t.attempts.map((a) => (
                                <div key={a.attempt_id} className="text-[11px] text-on-surface-variant flex items-center gap-2">
                                  <Icon name={a.outcome === "succeeded" ? "task_alt" : "sync_problem"} size={13} className={a.outcome === "succeeded" ? "text-emerald-600" : "text-amber-600"} />
                                  <span className="capitalize">{a.outcome}</span>
                                  <span>{fmtTime(a.created_at_ms)}</span>
                                  {a.failure_class && <span className="font-mono text-red-600 dark:text-red-400">{a.failure_class}</span>}
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                      </div>
                    </li>
                  );
                })}
              </ol>
            )}
          </div>

          {/* Activity timeline */}
          <div className="neo-raised rounded-2xl p-6 flex flex-col gap-2">
            <h2 className="text-lg font-semibold flex items-center gap-2 text-on-surface mb-2">
              <Icon name="timeline" size={20} className="text-primary" />
              Activity
            </h2>
            {renderedEvents.length === 0 ? (
              <p className="text-sm text-on-surface-variant">No activity recorded yet.</p>
            ) : (
              <ol className="relative space-y-3 pl-1">
                {renderedEvents.map((ev) => {
                  const meta = eventMeta(ev.kind);
                  return (
                    <li key={ev.id} className="flex items-start gap-3">
                      <span className={`mt-0.5 ${meta.cls}`}>
                        <Icon name={meta.icon} size={18} fill />
                      </span>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-baseline justify-between gap-2">
                          <p className="text-sm font-semibold text-on-surface">{meta.label}</p>
                          <span className="text-xs text-on-surface-variant whitespace-nowrap">{fmtTime(ev.created_at_ms)}</span>
                        </div>
                        {ev.detail && ev.detail !== "attempt recorded" && ev.detail !== "changeset recorded" && (
                          <p className="text-xs text-on-surface-variant mt-0.5 truncate">{ev.detail}</p>
                        )}
                      </div>
                    </li>
                  );
                })}
              </ol>
            )}
          </div>

  
          {/* Evidence */}
          <div className="neo-raised rounded-2xl p-6 flex flex-col gap-2">
            <h2 className="text-lg font-semibold flex items-center gap-2 text-on-surface mb-2">
              <Icon name="inventory_2" size={20} className="text-primary" />
              Evidence
              <button
                onClick={async () => {
                  if (whyOpen) {
                    setWhyOpen(false);
                    return;
                  }
                  setWhyBusy(true);
                  const chain = await daemon.evidenceChain(missionId!);
                  setWhyChain(chain);
                  setWhyBusy(false);
                  setWhyOpen(true);
                }}
                className="ml-auto text-xs text-primary hover:opacity-80"
                title="Walk the full chain: every task, its attempts, the evidence captured, and the audits that consumed it"
              >
                {whyBusy ? "Loading chain…" : "Why this result?"}
              </button>
            </h2>
            {whyOpen && whyChain && (
              <div className="rounded-xl neo-pressed p-3 mb-2 max-h-96 overflow-auto">
                <p className="text-xs text-on-surface-variant mb-2">
                  Task → attempt → captured evidence → consuming audits. Nothing here is a summary alone — every row traces to a recorded artifact.
                </p>
                {whyChain.tasks.map((t) => (
                  <div key={t.task_id} className="mb-2">
                    <p className="text-xs font-semibold text-on-surface">
                      {t.title} <span className="text-[10px] text-on-surface-variant">· {t.state}</span>
                    </p>
                    {t.attempts.map((a) => (
                      <div key={a.attempt_id} className="ml-3 mt-1">
                        <p className="text-[11px] text-on-surface-variant">
                          attempt · {a.outcome}
                          {a.failure_class ? ` · ${a.failure_class}` : ""}
                        </p>
                        {a.evidence.map((ev) => (
                          <p key={ev.evidence_id} className="ml-4 text-[10px] font-mono text-on-surface-variant">
                            ↳ {ev.kind} · {ev.source}
                            {ev.tool ? ` · ${ev.tool}` : ""} · {ev.content_hash.slice(0, 10)}…
                          </p>
                        ))}
                      </div>
                    ))}
                  </div>
                ))}
                {whyChain.final_audits.length > 0 && (
                  <div className="mt-2 border-t border-black/5 dark:border-white/5 pt-2">
                    {whyChain.final_audits.map((a) => (
                      <p key={a.audit_id} className="text-[11px] text-on-surface-variant">
                        Final audit · {a.passed ? "PASSED" : "REJECTED"}
                        {a.finding_codes.length > 0 ? ` · findings: ${a.finding_codes.join(", ")}` : " · no findings"}
                        {a.remaining_uncertainty ? ` · uncertainty: ${a.remaining_uncertainty}` : ""}
                      </p>
                    ))}
                  </div>
                )}
              </div>
            )}
            {evidence.length === 0 ? (
              <p className="text-sm text-on-surface-variant">No evidence recorded yet.</p>
            ) : (
              <div className="space-y-2">
                {evidence.map((item) => (
                  <div key={item.evidence_id} className="rounded-xl neo-pressed p-3">
                    <div className="flex items-center justify-between gap-2 flex-wrap">
                      <div className="flex items-center gap-2">
                        <Icon name={item.sensitive ? "lock" : "description"} size={15} className={item.sensitive ? "text-amber-600" : "text-primary"} />
                        <span className="text-sm font-semibold text-on-surface">{item.kind}</span>
                        {item.sensitive && <span className="text-[10px] px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-600 font-medium">Sensitive</span>}
                      </div>
                      <span className="text-xs text-on-surface-variant whitespace-nowrap">{fmtTime(item.created_at_ms)}</span>
                    </div>
                    {item.summary && <p className="text-xs text-on-surface-variant mt-1.5">{item.summary}</p>}
                    <p className="text-[11px] text-on-surface-variant mt-1.5 flex items-center gap-2">
                      <span className="font-mono">{item.provenance_source}</span>
                      {item.provenance_tool && <span className="font-mono">· {item.provenance_tool}</span>}
                      <span className="font-mono">· {item.content_hash.slice(0, 12)}…</span>
                    </p>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Right column */}
        <div className="flex flex-col gap-6">
          {/* Changed files / ChangeSet */}
          <div className="neo-raised rounded-2xl p-5 flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-bold text-on-surface-variant uppercase tracking-wider">Changes</h3>
              {changesets.length > 0 && (
                <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                  changesets.some((c) => c.applied) ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" : "bg-amber-500/10 text-amber-600"
                }`}>
                  {changesets.some((c) => c.applied) ? "Applied" : "Proposed"}
                </span>
              )}
            </div>
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
                      <span className="text-[10px] text-on-surface-variant">{fmtTime(cs.created_at_ms)}</span>
                    </div>
                    {cs.files.length > 0 && (
                      <div className="mt-2 space-y-1">
                        {cs.files.map((f) => (
                          <div key={f.path} className="flex items-center justify-between gap-2">
                            <span className="text-[11px] font-mono text-on-surface truncate">{f.path}</span>
                            <span className="text-[10px] flex gap-1.5 shrink-0">
                              <span className="text-emerald-600 dark:text-emerald-400">+{f.additions}</span>
                              <span className="text-red-600 dark:text-red-400">-{f.removals}</span>
                            </span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Verification */}
          <div className="neo-raised rounded-2xl p-5 flex flex-col gap-3">
            <h3 className="text-sm font-bold text-on-surface-variant uppercase tracking-wider">Verification</h3>
            {verification && verification.verifications.length > 0 ? (
              <div className="space-y-2">
                {verification.verifications.map((run) => (
                  <div key={run.verification_id} className="neo-pressed rounded-xl p-3">
                    <div className="flex items-center justify-between gap-2">
                      <span className="flex items-center gap-1.5 text-sm font-semibold text-on-surface">
                        <Icon name={run.passed ? "verified_user" : "report_problem"} size={16} className={run.passed ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"} />
                        {run.passed ? "Passed" : run.status}
                      </span>
                      <span className="text-[10px] text-on-surface-variant">{fmtTime(run.created_at_ms)}</span>
                    </div>
                    <p className="text-[11px] text-on-surface-variant mt-1 font-mono truncate">{run.command}</p>
                    <p className="text-[10px] text-on-surface-variant mt-0.5">{run.environment}</p>
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
            <button onClick={onOpenSettings} className="text-primary font-medium text-xs hover:underline mt-1 text-left">
              Open Settings
            </button>
          </div>
        </div>
      </div>
    </main>
  );
}