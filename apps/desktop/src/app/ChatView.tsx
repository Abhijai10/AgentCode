import { useEffect, useState, useRef, useCallback } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { Project } from "./ProjectContext";
import type {
  Conversation,
  ConversationDetail,
  ConversationActivity,
  ConversationActivityMission,
  Message,
  Attachment,
  ConversationMode,
  MissionDetails,
  TaskDetail,
  MissionActivityEvent,
  ChangeSetSummary,
  EvidenceSummaryItem,
  VerificationSummary,
} from "./types";

const POLL_INTERVAL_MS = 2500;
const MAX_VISIBLE_EVENTS = 40;

function modeLabel(mode: ConversationMode): string {
  switch (mode) {
    case "GOAL": return "Goal";
    case "DISCUSS": return "Discuss";
    case "DESIGN": return "Design";
    case "SECURITY": return "Security";
  }
}

function modeColor(mode: ConversationMode): string {
  switch (mode) {
    case "GOAL": return "text-primary";
    case "DISCUSS": return "text-violet-600 dark:text-violet-400";
    case "DESIGN": return "text-amber-600 dark:text-amber-400";
    case "SECURITY": return "text-emerald-600 dark:text-emerald-400";
  }
}

function formatTime(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function formatDate(ms: number): string {
  const d = new Date(ms);
  const now = new Date();
  if (d.toDateString() === now.toDateString()) return formatTime(ms);
  return d.toLocaleDateString([], { month: "short", day: "numeric" }) + " " + formatTime(ms);
}

// ── Mission state helpers (shared with the inspector's vocabulary) ───────────

function isTerminalState(state: string | undefined | null): boolean {
  if (!state) return false;
  return state === "completed" || state === "cancelled" || state === "failed" || state.startsWith("failed:");
}

function stateLabel(state: string | undefined | null): string {
  switch (state) {
    case "queued": return "Queued";
    case "pending": return "Pending";
    case "ready": return "Ready";
    case "running": return "Running";
    case "retryable": return "Retryable";
    case "paused": return "Paused";
    case "completed": return "Completed";
    case "cancelled": return "Cancelled";
    default:
      if (state?.startsWith("failed:")) return "Failed";
      if (state === "failed") return "Failed";
      if (!state) return "—";
      return state.charAt(0).toUpperCase() + state.slice(1);
  }
}

function stateColor(state: string | undefined | null): string {
  if (state === "completed") return "text-emerald-600 dark:text-emerald-400";
  if (state === "cancelled") return "text-on-surface-variant";
  if (state === "failed" || state?.startsWith("failed:")) return "text-red-600 dark:text-red-400";
  if (state === "paused") return "text-amber-600 dark:text-amber-400";
  if (state === "running" || state === "retryable" || state === "queued") return "text-primary";
  return "text-on-surface-variant";
}

function taskStateIcon(state: string): { icon: string; cls: string } {
  if (state === "completed") return { icon: "check_circle", cls: "text-emerald-600 dark:text-emerald-400" };
  if (state === "running" || state === "ready") return { icon: "autorenew", cls: "text-primary" };
  if (state === "failed" || state === "retryable") return { icon: "error", cls: "text-red-600 dark:text-red-400" };
  return { icon: "radio_button_unchecked", cls: "text-on-surface-variant" };
}

const EVENT_META: Record<string, { label: string; icon: string; cls: string }> = {
  "kernel.create_mission": { label: "Mission created", icon: "add_circle", cls: "text-primary" },
  "kernel.activate_mission": { label: "Mission started", icon: "play_arrow", cls: "text-primary" },
  "kernel.complete_mission": { label: "Mission completed", icon: "check_circle", cls: "text-emerald-600 dark:text-emerald-400" },
  "kernel.cancel_mission": { label: "Mission cancelled", icon: "cancel", cls: "text-on-surface-variant" },
  "kernel.approve_changeset": { label: "ChangeSet approved", icon: "verified_user", cls: "text-primary" },
};

function eventMeta(kind: string): { label: string; icon: string; cls: string } {
  const direct = EVENT_META[kind];
  if (direct) return direct;
  if (kind.startsWith("attempt.")) {
    const outcome = kind.replace("attempt.", "");
    const ok = outcome === "succeeded";
    return {
      label: ok ? "Task attempt succeeded" : `Task attempt ${outcome}`,
      icon: ok ? "task_alt" : "sync_problem",
      cls: ok ? "text-emerald-600 dark:text-emerald-400" : "text-amber-600 dark:text-amber-400",
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
  return { label: kind.replace(/_/g, " ").replace(/\./g, " · "), icon: "info", cls: "text-on-surface-variant" };
}

// ── Expandable section container ─────────────────────────────────────────────

function Section({
  title,
  icon,
  badge,
  defaultOpen = false,
  children,
}: {
  title: string;
  icon: string;
  badge?: ReactNode;
  defaultOpen?: boolean;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="border-t border-outline-variant/30 dark:border-white/5 first:border-t-0">
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left text-xs font-medium text-on-surface hover:bg-surface-variant/30 rounded-md"
        aria-expanded={open}
      >
        <Icon name="expand_more" size={14} className={`transition-transform ${open ? "" : "-rotate-90"}`} />
        <Icon name={icon} size={14} className="text-on-surface-variant" />
        <span>{title}</span>
        {badge && <span className="ml-auto">{badge}</span>}
      </button>
      {open && <div className="px-3 pb-2 pl-9">{children}</div>}
    </div>
  );
}

function MissionBlock({
  mission,
  onOpenMission,
}: {
  mission: ConversationActivityMission;
  onOpenMission(missionId: string): void;
}) {
  const details: MissionDetails = mission.details;
  const state = mission.status ?? details?.state;
  const terminal = isTerminalState(state);
  const tasks: TaskDetail[] = mission.tasks?.tasks ?? [];
  const events: MissionActivityEvent[] = mission.events?.events ?? [];
  const changesets: ChangeSetSummary[] = mission.changesets?.changesets ?? [];
  const evidence: EvidenceSummaryItem[] = mission.evidence?.evidence ?? [];
  const verification: VerificationSummary = mission.verification ?? { verifications: [], final_audits: [] };
  const summary = mission.summary;

  const files = changesets.flatMap((cs) => cs.files ?? []);
  const totalAdd = files.reduce((sum, f) => sum + (f.additions ?? 0), 0);
  const totalDel = files.reduce((sum, f) => sum + (f.removals ?? 0), 0);

  const planDone = tasks.filter((t) => t.state === "completed").length;
  const planFailed = tasks.filter((t) => t.state === "failed" || t.state === "retryable").length;
  const visibleEvents = events.slice(0, MAX_VISIBLE_EVENTS);
  const truncatedEvents = events.length - visibleEvents.length;

  // Factual failure detail (only from authoritative state — never fabricated).
  const failureClasses = summary?.failure_classes ?? [];
  const failedTaskCount = summary?.failed_task_count ?? 0;
  const retryCount = summary?.retry_count ?? 0;
  const toolsUsed = summary?.tools_used ?? [];
  const commands = summary?.commands ?? [];
  const hasFailures = failedTaskCount > 0 || failureClasses.length > 0;

  return (
    <div className="neo-pressed rounded-xl overflow-hidden text-xs mt-2">
      {/* Header: status + actions */}
      <div className="flex items-center gap-2 px-3 py-2 bg-surface/60">
        <Icon name="terminal" size={14} className="text-primary" />
        <span className="font-semibold text-on-surface truncate">Mission</span>
        <span className={`flex items-center gap-1 font-medium ${stateColor(state)}`}>
          {!terminal && state === "running" && <Icon name="autorenew" size={12} className="animate-spin" />}
          {stateLabel(state)}
        </span>
        <span className="text-on-surface-variant text-[10px] truncate ml-1">
          {details?.goal ?? mission.mission_id}
        </span>
        <button
          onClick={() => onOpenMission(mission.mission_id)}
          className="ml-auto neo-button px-2 py-1 rounded-md text-[10px] text-primary font-medium shrink-0"
          aria-label={`Open mission ${mission.mission_id} details`}
        >
          Details
        </button>
      </div>

      {/* Result summary (terminal only) */}
      {terminal && (
        <div className="flex items-center gap-2 px-3 py-2 border-t border-outline-variant/30 dark:border-white/5">
          <Icon
            name={state === "completed" ? "check_circle" : state === "cancelled" ? "cancel" : "error"}
            size={14}
            className={stateColor(state)}
          />
          <span className={`font-medium ${stateColor(state)}`}>
            {state === "completed" ? "Completed" : state === "cancelled" ? "Cancelled" : "Failed"}
          </span>
          {details?.failure_code && (
            <span className="text-red-600 dark:text-red-400 text-[10px] font-mono truncate">
              {details.failure_code}
            </span>
          )}
          {verification.final_audits?.length > 0 && (
            <span className="text-on-surface-variant text-[10px] ml-auto">
              {verification.final_audits[verification.final_audits.length - 1].passed
                ? "Completion gate passed"
                : "Completion gate not passed"}
            </span>
          )}
        </div>
      )}

      {/* Failure / recovery (factual, only when authoritative state exists) */}
      {hasFailures && (
        <div className="px-3 py-2 border-t border-outline-variant/30 dark:border-white/5">
          <div className="flex items-center gap-2 text-red-600 dark:text-red-400">
            <Icon name="error" size={14} />
            <span className="font-medium">Failures</span>
            {failureClasses.map((cls) => (
              <span key={cls} className="text-[10px] font-mono bg-surface-variant/40 rounded px-1.5 py-0.5">
                {cls}
              </span>
            ))}
          </div>
          <div className="text-[10px] text-on-surface-variant mt-1">
            {failedTaskCount} task{failedTaskCount === 1 ? "" : "s"} failed
            {retryCount > 0 && ` · ${retryCount} retr${retryCount === 1 ? "y" : "ies"} performed`}
          </div>
        </div>
      )}

      {/* Plan */}
      {tasks.length > 0 && (
        <Section
          title={`Plan · ${planDone}/${tasks.length} done${planFailed > 0 ? ` · ${planFailed} failed` : ""}`}
          icon="list_alt"
          badge={
            <span className="text-[10px] text-on-surface-variant">{terminal ? "Done" : "Running"}</span>
          }
        >
          <ul className="space-y-1">
            {tasks.map((task) => {
              const meta = taskStateIcon(task.state);
              return (
                <li key={task.task_id} className="flex items-start gap-2">
                  <Icon name={meta.icon} size={13} className={`mt-0.5 ${meta.cls} ${task.state === "running" || task.state === "ready" ? "animate-spin" : ""}`} />
                  <span className="text-on-surface break-words">{task.title || task.task_id}</span>
                  {task.retry_count > 0 && (
                    <span className="text-[10px] text-amber-600 dark:text-amber-400 shrink-0">retry {task.retry_count}</span>
                  )}
                </li>
              );
            })}
          </ul>
        </Section>
      )}

      {/* Tools / Commands (factual execution summary) */}
      {(toolsUsed.length > 0 || commands.length > 0) && (
        <Section
          title="Tools & Commands"
          icon="construction"
          badge={<span className="text-[10px] text-on-surface-variant">{toolsUsed.length} tool{toolsUsed.length === 1 ? "" : "s"}</span>}
        >
          {toolsUsed.length > 0 && (
            <div className="flex flex-wrap gap-1 mb-1">
              {toolsUsed.map((tool) => (
                <span key={tool} className="text-[10px] font-mono text-on-surface-variant neo-pressed rounded px-1.5 py-0.5">
                  {tool}
                </span>
              ))}
            </div>
          )}
          {commands.length > 0 && (
            <ul className="space-y-0.5">
              {commands.slice(0, 20).map((command, i) => (
                <li key={i} className="font-mono text-[10px] text-on-surface truncate">
                  $ {command}
                </li>
              ))}
            </ul>
          )}
        </Section>
      )}

      {/* Activity */}
      {events.length > 0 && (
        <Section title={`Activity · ${events.length} event${events.length === 1 ? "" : "s"}`} icon="bolt">
          <ul className="space-y-1">
            {visibleEvents.map((event) => {
              const meta = eventMeta(event.kind);
              return (
                <li key={event.id} className="flex items-start gap-2">
                  <Icon name={meta.icon} size={13} className={`mt-0.5 ${meta.cls}`} />
                  <div className="min-w-0">
                    <span className="text-on-surface">{meta.label}</span>
                    {event.detail && (
                      <span className="text-on-surface-variant block text-[10px] break-words">{event.detail}</span>
                    )}
                  </div>
                </li>
              );
            })}
            {truncatedEvents > 0 && (
              <li className="text-on-surface-variant text-[10px] pl-5">… {truncatedEvents} more</li>
            )}
          </ul>
        </Section>
      )}

      {/* Changes */}
      {files.length > 0 && (
        <Section title={`Changes · ${files.length} file${files.length === 1 ? "" : "s"}`} icon="difference">
          <div className="text-[10px] text-on-surface-variant mb-1">
            +{totalAdd} / −{totalDel}
          </div>
          <ul className="space-y-0.5">
            {files.slice(0, 40).map((f, i) => (
              <li key={`${f.path}-${i}`} className="font-mono text-[10px] text-on-surface truncate">
                {f.path}
                <span className="text-on-surface-variant"> ({f.strategy})</span>
              </li>
            ))}
          </ul>
        </Section>
      )}

      {/* Verification */}
      {(verification.verifications?.length > 0 || verification.final_audits?.length > 0) && (
        <Section title="Verification" icon="verified_user">
          <ul className="space-y-1">
            {verification.verifications.map((run) => (
              <li key={run.verification_id} className="flex items-start gap-2">
                <Icon
                  name={run.passed ? "check_circle" : "error"}
                  size={13}
                  className={`mt-0.5 ${run.passed ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}
                />
                <div className="min-w-0">
                  <span className="text-on-surface">{run.passed ? "Passed" : "Failed"}</span>
                  {run.command && (
                    <span className="text-on-surface-variant block text-[10px] font-mono truncate">{run.command}</span>
                  )}
                </div>
              </li>
            ))}
            {verification.final_audits.map((audit) => (
              <li key={audit.audit_id} className="flex items-start gap-2">
                <Icon name={audit.passed ? "verified_user" : "report_problem"} size={13} className={`mt-0.5 ${audit.passed ? "text-emerald-600 dark:text-emerald-400" : "text-amber-600 dark:text-amber-400"}`} />
                <span className="text-on-surface">
                  {audit.passed ? "Completion audit passed" : "Completion audit not passed"}
                </span>
              </li>
            ))}
          </ul>
        </Section>
      )}

      {/* Evidence */}
      {evidence.length > 0 && (
        <Section title={`Evidence · ${evidence.length}`} icon="inventory_2">
          <ul className="space-y-0.5">
            {evidence.slice(0, 30).map((item) => (
              <li key={item.evidence_id} className="flex items-start gap-2">
                <Icon name="inventory_2" size={13} className="mt-0.5 text-on-surface-variant" />
                <div className="min-w-0">
                  <span className="text-on-surface">{item.kind.replace(/_/g, " ")}</span>
                  {item.summary && (
                    <span className="text-on-surface-variant block text-[10px] break-words">{item.summary}</span>
                  )}
                </div>
              </li>
            ))}
          </ul>
        </Section>
      )}

      {/* Running state note */}
      {!terminal && (
        <div className="flex items-center gap-2 px-3 py-2 border-t border-outline-variant/30 dark:border-white/5 text-on-surface-variant text-[10px]">
          <Icon name="autorenew" size={12} className="animate-spin" />
          {verification.verifications?.length > 0 && !verification.final_audits?.length
            ? "Verification running…"
            : "Working on this…"}
        </div>
      )}
    </div>
  );
}

export function ChatView({
  project,
  onOpenMission,
  daemonConnected,
}: {
  project: Project | null;
  onOpenMission(missionId: string): void;
  daemonConnected: boolean;
}) {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [listCollapsed, setListCollapsed] = useState(false);
  const [activeConvId, setActiveConvId] = useState<string | null>(null);
  const [convDetail, setConvDetail] = useState<ConversationDetail | null>(null);
  const [activity, setActivity] = useState<ConversationActivity | null>(null);
  const [input, setInput] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pendingAttachments, setPendingAttachments] = useState<Attachment[]>([]);
  const [attachments, setAttachments] = useState<Attachment[]>([]);
  const [status, setStatus] = useState<"no_project" | "loading" | "loaded" | "daemon_unavailable" | "no_chat">(
    project ? "loading" : "no_project"
  );
  const [renameTarget, setRenameTarget] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Load conversations when project changes
  useEffect(() => {
    if (!project) {
      setStatus("no_project");
      setConversations([]);
      setActiveConvId(null);
      setConvDetail(null);
      setActivity(null);
      return;
    }
    setStatus("loading");
    let cancelled = false;
    (async () => {
      const convs = await daemon.listConversations(project.path);
      if (cancelled) return;
      setConversations(convs);
      if (!activeConvId && convs.length > 0) {
        setActiveConvId(convs[0].id);
      }
      setStatus(convs.length > 0 ? "loaded" : "no_chat");
    })();
    return () => { cancelled = true; };
  }, [project]);

  // Load conversation detail + activity when active conversation changes
  useEffect(() => {
    if (!activeConvId) {
      setConvDetail(null);
      setActivity(null);
      setAttachments([]);
      return;
    }
    let cancelled = false;
    (async () => {
      const [detail, act] = await Promise.all([
        daemon.getConversation(activeConvId),
        daemon.getConversationActivity(activeConvId),
      ]);
      if (cancelled) return;
      if (detail) {
        setConvDetail(detail);
        setAttachments(detail.attachments ?? []);
      }
      setActivity(act);
    })();
    return () => { cancelled = true; };
  }, [activeConvId]);

  // Scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [convDetail?.messages?.length]);

  // Live refresh while any mission in this conversation is non-terminal.
  // Doc 06 H4 event-push: EventsSubscribe LONG-POLLS the daemon — one
  // cheap blocked request that returns the moment a kernel event lands,
  // instead of a timer hammering the cursor.  Each event batch triggers
  // exactly one heavy refetch (cursor still gates the payload work);
  // when the stream is unavailable (older daemon) the loop falls back to
  // the F11 cursor interval.  Stops when all missions reach a terminal
  // state; the terminal result stays visible from the last refresh.
  useEffect(() => {
    if (!activeConvId) return;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | null = null;
    let lastCursor: string | null = null;

    const refetchHeavy = async () => {
      const cursor = await daemon.conversationChangesCursor(activeConvId);
      if (cancelled) return false;
      if (cursor) {
        const signature = JSON.stringify([
          cursor.updated_at_ms,
          cursor.message_count,
          cursor.latest_message_id,
          cursor.missions,
        ]);
        if (signature === lastCursor) return false; // nothing changed — skip heavy work
        lastCursor = signature;
      }
      const [detail, act] = await Promise.all([
        daemon.getConversation(activeConvId),
        daemon.getConversationActivity(activeConvId),
      ]);
      if (cancelled) return false;
      if (detail) {
        setConvDetail(detail);
        setAttachments(detail.attachments ?? []);
      }
      setActivity(act);
      const anyRunning = (act?.missions ?? []).some((m) => !isTerminalState(m.status ?? m.details?.state));
      return anyRunning;
    };

    // Fallback path: the interval-poll of the F11 cursor (older daemons
    // without EventsSubscribe, or a subscribe error).
    const cursorLoop = async () => {
      const anyRunning = await refetchHeavy();
      if (cancelled || !anyRunning) return;
      timer = setTimeout(cursorLoop, POLL_INTERVAL_MS);
    };

    // Primary path: subscribe to the kernel event stream.
    const subscribeLoop = async (cursorMs: number, cursorId: string) => {
      if (cancelled) return;
      const res = await daemon.eventsSubscribe(cursorMs, cursorId, 5000);
      if (cancelled) return;
      if (!res) {
        // Stream unavailable — fall back to the interval path, forever.
        cursorLoop();
        return;
      }
      if (res.events.length > 0) {
        const anyRunning = await refetchHeavy();
        if (cancelled) return;
        if (!anyRunning) return; // all terminal — stop streaming
      }
      timer = setTimeout(
        () => subscribeLoop(res.cursor.created_at_ms, res.cursor.id),
        50
      );
    };

    const run = async () => {
      const act = await daemon.getConversationActivity(activeConvId);
      if (cancelled) return;
      setActivity(act);
      const anyRunning = (act?.missions ?? []).some((m) => !isTerminalState(m.status ?? m.details?.state));
      if (!anyRunning) return;
      // Start at the current tail: events from now on.
      subscribeLoop(-1, "");
    };
    run();
    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
    };
  }, [activeConvId]);

  const refreshConversations = useCallback(async () => {
    if (!project) return;
    const convs = await daemon.listConversations(project.path);
    setConversations(convs);
  }, [project]);

  const refreshActive = useCallback(async () => {
    if (!activeConvId) return;
    const [detail, act] = await Promise.all([
      daemon.getConversation(activeConvId),
      daemon.getConversationActivity(activeConvId),
    ]);
    if (detail) {
      setConvDetail(detail);
      setAttachments(detail.attachments ?? []);
    }
    setActivity(act);
  }, [activeConvId]);

  const handleNewChat = async () => {
    if (!project) return;
    setBusy(true);
    setError(null);
    const result = await daemon.createConversation(project.path, "GOAL", "New Chat");
    if (result.ok && result.conversation_id) {
      setActiveConvId(result.conversation_id);
      await refreshConversations();
      setStatus("loaded");
    } else {
      setError(result.error || "Could not create conversation");
    }
    setBusy(false);
  };

  const handleSend = async () => {
    const trimmed = input.trim();
    if (!trimmed || !activeConvId || submitting) return;
    setSubmitting(true);
    setError(null);
    const result = await daemon.appendMessage(activeConvId, "user", trimmed);
    if (result.ok) {
      setInput("");
      await refreshActive();
    } else {
      setError(result.error || "Could not send message");
    }
    setSubmitting(false);
  };

  const handleAttach = async () => {
    if (!project || !activeConvId) return;
    setBusy(true);
    setError(null);
    const result = await daemon.addAttachment(activeConvId, project.path);
    if (result.ok && result.attachment) {
      setPendingAttachments((prev) => [...prev, result.attachment!]);
    } else {
      setError(result.error || "Could not attach file");
    }
    setBusy(false);
  };

  const handleRemovePending = (id: string) => {
    setPendingAttachments((prev) => prev.filter((a) => a.id !== id));
  };

  const handleRunMission = async () => {
    if (!activeConvId || !convDetail) return;
    // Use the latest user message as the goal; if the typed input is newer,
    // prefer it so follow-up goals after a terminal mission work naturally.
    const lastUserMsg = convDetail.messages
      .filter((m) => m.role === "user" && !m.mission_ref)
      .pop();
    const goal = input.trim() || lastUserMsg?.content || "";
    if (!goal) return;
    setBusy(true);
    setError(null);
    const result = await daemon.submitGoalFromConversation(
      activeConvId,
      goal,
      pendingAttachments.map((a) => a.id)
    );
    if (result.ok) {
      setInput("");
      setPendingAttachments([]);
      await refreshActive();
      await refreshConversations();
      onOpenMission(result.mission_id);
    } else {
      setError(result.error || "Could not submit mission");
    }
    setBusy(false);
  };

  const handleRename = async (id: string) => {
    const trimmed = renameValue.trim();
    if (!trimmed) { setRenameTarget(null); return; }
    await daemon.renameConversation(id, trimmed);
    setRenameTarget(null);
    await refreshConversations();
  };

  const handleArchive = async (id: string) => {
    await daemon.archiveConversation(id);
    if (activeConvId === id) setActiveConvId(null);
    await refreshConversations();
  };

  const handleDelete = async (id: string) => {
    await daemon.deleteConversation(id);
    if (activeConvId === id) setActiveConvId(null);
    await refreshConversations();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // ── Render helpers ──────────────────────────────────────────────────────────

  function renderMessage(msg: Message) {
    const isUser = msg.role === "user";
    const linkAttachments = attachments.filter((a) => a.message_id === msg.id);
    const mission = msg.mission_ref
      ? activity?.missions.find((m) => m.mission_id === msg.mission_ref)
      : undefined;
    return (
      <div key={msg.id} className="mb-4">
        <div className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
          <div className={`max-w-[85%] ${isUser ? "order-1" : "order-1"}`}>
            <div
              className={`rounded-2xl px-4 py-3 ${
                isUser
                  ? "bg-primary text-on-primary rounded-br-md"
                  : "neo-pressed text-on-surface rounded-bl-md"
              }`}
            >
              <p className="text-sm whitespace-pre-wrap break-words">{msg.content}</p>
              {msg.mission_ref && (
                <div className="mt-2 flex items-center gap-2 text-xs opacity-70">
                  <Icon name="terminal" size={12} />
                  Mission: {msg.mission_ref}
                  <button
                    onClick={() => onOpenMission(msg.mission_ref!)}
                    className="underline hover:opacity-80"
                  >
                    View
                  </button>
                </div>
              )}
            </div>
            {linkAttachments.length > 0 && (
              <div className="mt-1 flex flex-wrap gap-1.5">
                {linkAttachments.map((a) => (
                  <span key={a.id} className="text-[10px] text-on-surface-variant neo-pressed rounded px-1.5 py-0.5">
                    {a.filename}
                  </span>
                ))}
              </div>
            )}
            <p className="text-[10px] text-on-surface-variant mt-0.5 px-1">
              {formatDate(msg.created_at_ms)}
            </p>
          </div>
        </div>
        {mission && (
          <div className="mt-1 max-w-[85%]">
            <MissionBlock mission={mission} onOpenMission={onOpenMission} />
          </div>
        )}
      </div>
    );
  }

  function renderAttachmentPreview(a: Attachment) {
    const isImage = a.mime_type.startsWith("image/");
    return (
      <div
        key={a.id}
        className="neo-pressed rounded-xl p-2 flex items-center gap-2 text-xs"
      >
        <Icon name={isImage ? "image" : "description"} size={16} className="text-primary" />
        <span className="truncate max-w-[120px]">{a.filename}</span>
        <span className="text-on-surface-variant shrink-0">
          {a.size_bytes > 1024
            ? `${(a.size_bytes / 1024).toFixed(0)}KB`
            : `${a.size_bytes}B`}
        </span>
        <button
          onClick={() => handleRemovePending(a.id)}
          className="text-red-500 hover:text-red-600 ml-auto"
        >
          <Icon name="close" size={14} />
        </button>
      </div>
    );
  }

  // ── Main render ─────────────────────────────────────────────────────────────

  if (status === "no_project") {
    return (
      <main className="flex-1 flex items-center justify-center">
        <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
          <Icon name="forum" size={40} className="text-primary mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-on-surface mb-2">Open a Project</h3>
          <p className="text-sm text-on-surface-variant">
            Select a project to view and manage your conversations.
          </p>
        </div>
      </main>
    );
  }

  if (status === "daemon_unavailable") {
    return (
      <main className="flex-1 flex items-center justify-center">
        <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
          <Icon name="dns_off" size={40} className="text-red-500 mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-on-surface mb-2">Daemon Unavailable</h3>
          <p className="text-sm text-on-surface-variant">
            The AgentCode daemon is not responding. Conversations are persisted and will be available when the daemon restarts.
          </p>
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 flex overflow-hidden">
      {/* Conversation List */}
      {listCollapsed ? (
      <button
        onClick={() => setListCollapsed(false)}
        title="Show list"
        className="shrink-0 w-8 border-r border-outline-variant/40 dark:border-white/5 bg-surface/50 flex items-center justify-center text-on-surface-variant hover:text-primary"
      >
        <Icon name="arrow_forward" size={16} />
      </button>
    ) : (
    <aside className="w-64 shrink-0 flex flex-col border-r border-outline-variant/40 dark:border-white/5 bg-surface/50 relative">
        <button
          onClick={() => setListCollapsed(true)}
          title="Collapse list"
          className="absolute top-2 right-2 z-10 p-1 rounded-lg text-on-surface-variant hover:bg-surface-variant/40 dark:hover:bg-white/5"
        >
          <Icon name="arrow_back" size={14} className="rotate-180" />
        </button>
        <div className="p-3 border-b border-outline-variant/40 dark:border-white/5">
          <button
            onClick={handleNewChat}
            disabled={busy}
            className="w-full neo-button rounded-xl py-2.5 flex items-center justify-center gap-2 text-sm font-medium text-primary"
          >
            <Icon name="add" size={18} />
            New Chat
          </button>
        </div>
        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          {conversations.map((conv) => (
            <div key={conv.id}>
              <button
                onClick={() => setActiveConvId(conv.id)}
                className={`w-full text-left rounded-xl px-3 py-2.5 transition-colors ${
                  activeConvId === conv.id
                    ? "neo-pressed text-primary"
                    : "hover:bg-surface-variant/40 text-on-surface-variant"
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className={`text-[10px] font-bold uppercase ${modeColor(conv.mode as ConversationMode)}`}>
                    {modeLabel(conv.mode as ConversationMode)}
                  </span>
                </div>
                <p className="text-sm font-medium truncate mt-0.5">
                  {renameTarget === conv.id ? (
                    <input
                      className="neo-input rounded px-1.5 py-0.5 text-xs w-full"
                      value={renameValue}
                      onChange={(e) => setRenameValue(e.target.value)}
                      onBlur={() => handleRename(conv.id)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleRename(conv.id);
                        if (e.key === "Escape") setRenameTarget(null);
                      }}
                      autoFocus
                      onClick={(e) => e.stopPropagation()}
                    />
                  ) : (
                    conv.title
                  )}
                </p>
                <p className="text-[10px] text-on-surface-variant mt-0.5">
                  {formatDate(conv.updated_at_ms)}
                </p>
              </button>
              <div className="flex gap-1 px-3 mb-1">
                <button
                  onClick={() => { setRenameTarget(conv.id); setRenameValue(conv.title); }}
                  className="text-[10px] text-on-surface-variant hover:text-primary"
                >
                  Rename
                </button>
                <span className="text-on-surface-variant/30">·</span>
                <button
                  onClick={() => handleArchive(conv.id)}
                  className="text-[10px] text-on-surface-variant hover:text-primary"
                >
                  Archive
                </button>
                <span className="text-on-surface-variant/30">·</span>
                <button
                  onClick={() => handleDelete(conv.id)}
                  className="text-[10px] text-red-500 hover:text-red-600"
                >
                  Delete
                </button>
              </div>
            </div>
          ))}
          {conversations.length === 0 && (
            <div className="text-center text-xs text-on-surface-variant py-8">
              No conversations yet. Start a new chat.
            </div>
          )}
        </div>
      </aside>
    )}

      {/* Chat Area */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {!activeConvId ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
              <Icon name="forum" size={40} className="text-primary mx-auto mb-4" />
              <h3 className="text-lg font-semibold text-on-surface mb-2">Select a Conversation</h3>
              <p className="text-sm text-on-surface-variant">
                Choose a conversation from the sidebar or start a new chat.
              </p>
            </div>
          </div>
        ) : (
          <>
            {/* Header */}
            <div className="shrink-0 px-6 py-3 border-b border-outline-variant/40 dark:border-white/5 flex items-center justify-between">
              <div>
                <h2 className="text-lg font-semibold text-on-surface truncate">
                  {convDetail?.title || "Loading..."}
                </h2>
                {convDetail && (
                  <span className={`text-xs font-bold uppercase ${modeColor(convDetail.mode as ConversationMode)}`}>
                    {modeLabel(convDetail.mode as ConversationMode)} Mode
                  </span>
                )}
              </div>
              {daemonConnected ? (
                <span className="flex items-center gap-1.5 text-[10px] text-emerald-600 dark:text-emerald-400">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                  Connected
                </span>
              ) : (
                <span className="flex items-center gap-1.5 text-[10px] text-red-600 dark:text-red-400">
                  <span className="w-1.5 h-1.5 rounded-full bg-red-500" />
                  Daemon unavailable
                </span>
              )}
            </div>

            {/* Messages */}
            <div className="flex-1 overflow-y-auto px-6 py-4">
              {convDetail?.messages.map(renderMessage)}
              {!convDetail || convDetail.messages.length === 0 ? (
                <div className="text-center text-xs text-on-surface-variant py-10">
                  Type a message, attach files, then run it as a mission.
                </div>
              ) : null}
              <div ref={messagesEndRef} />
            </div>

            {/* Pending Attachments */}
            {pendingAttachments.length > 0 && (
              <div className="shrink-0 px-6 py-2 border-t border-outline-variant/40 dark:border-white/5">
                <div className="flex flex-wrap gap-2">
                  {pendingAttachments.map(renderAttachmentPreview)}
                </div>
              </div>
            )}

            {/* Composer */}
            <div className="shrink-0 px-6 py-4 border-t border-outline-variant/40 dark:border-white/5">
              {error && (
                <div className="flex items-center gap-2 text-xs text-red-600 dark:text-red-400 mb-2">
                  <Icon name="error" size={14} fill /> {error}
                </div>
              )}
              <div className="neo-raised rounded-[20px] p-2">
                <div className="neo-pressed rounded-[16px] px-4 py-3 flex items-end gap-2">
                  <textarea
                    ref={inputRef}
                    className="flex-1 bg-transparent border-none outline-none resize-none text-sm text-on-surface placeholder:text-on-surface-variant/50 max-h-[120px] focus:ring-0 p-0"
                    placeholder="Type a message..."
                    value={input}
                    disabled={submitting || busy}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                    rows={1}
                  />
                  <button
                    onClick={handleAttach}
                    disabled={busy}
                    className="w-9 h-9 rounded-full neo-button flex items-center justify-center text-on-surface-variant hover:text-primary"
                    title="Attach file"
                    aria-label="Attach file"
                  >
                    <Icon name="attach_file" size={18} />
                  </button>
                  <button
                    onClick={handleSend}
                    disabled={submitting || !input.trim()}
                    className="w-9 h-9 rounded-full bg-primary flex items-center justify-center text-on-primary disabled:opacity-50 active:scale-95 transition-all"
                    aria-label="Send message"
                  >
                    <Icon name={submitting ? "autorenew" : "send"} size={18} className={submitting ? "animate-spin" : ""} />
                  </button>
                </div>
              </div>
              {/* Run Mission button (GOAL mode) */}
              {convDetail?.mode === "GOAL" && (
                <div className="flex justify-end mt-2">
                  <button
                    onClick={handleRunMission}
                    disabled={busy || (!input.trim() && !convDetail?.messages?.some((m) => m.role === "user" && !m.mission_ref))}
                    className="neo-button rounded-xl px-4 py-2 text-sm font-medium text-primary flex items-center gap-2 disabled:opacity-50"
                    aria-label="Run as Mission"
                  >
                    <Icon name="terminal" size={16} />
                    Run as Mission
                  </button>
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </main>
  );
}
