import { useEffect, useState, useRef, useCallback } from "react";
import type { KeyboardEvent } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { Project } from "./ProjectContext";
import type {
  Conversation,
  ConversationDetail,
  Message,
  SecurityModeFinding,
  SecurityAttackPath,
  SecurityStatus,
  SecurityAuditResult,
  SecurityReportData,
  SecurityValidation,
  SecuritySecretLifecycle,
  SecurityQualityMetrics,
} from "./types";

function formatTime(ms: number): string {
  const d = new Date(ms);
  const now = new Date();
  if (d.toDateString() === now.toDateString()) {
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  return (
    d.toLocaleDateString([], { month: "short", day: "numeric" }) +
    " " +
    d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
  );
}

const SEVERITY_COLORS: Record<string, string> = {
  Critical: "bg-red-600/10 text-red-600 dark:text-red-400",
  High: "bg-orange-500/10 text-orange-600 dark:text-orange-400",
  Medium: "bg-amber-500/10 text-amber-600 dark:text-amber-400",
  Low: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400",
};

const STATE_COLORS: Record<string, string> = {
  New: "bg-on-surface-variant/10 text-on-surface-variant",
  Triaged: "bg-primary/10 text-primary",
  Validating: "bg-violet-500/10 text-violet-600 dark:text-violet-400",
  Confirmed: "bg-red-600/10 text-red-600 dark:text-red-400",
  Dismissed: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400",
  NeedsManualReview: "bg-amber-500/10 text-amber-600 dark:text-amber-400",
  Fixed: "bg-primary/10 text-primary",
  Retesting: "bg-violet-500/10 text-violet-600 dark:text-violet-400",
  Closed: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400",
};

const SCOPE_KIND_LABEL: Record<string, string> = {
  RepositoryOnly: "Repository Only",
  LocalOnly: "Local Only",
  StagingAuthorized: "Staging Authorized",
  ProductionReadOnly: "Production Read-Only",
  ProductionActiveApproved: "Production Active (Approved)",
  CloudLabAuthorized: "Cloud Lab Authorized",
};

const AUTH_LABEL: Record<string, string> = {
  ReadOnlyAudit: "Read-Only Audit",
  ActiveValidation: "Active Validation",
  AuthorizedAdversarial: "Authorized Adversarial",
};

export function SecurityView({
  project,
  daemonConnected,
  onOpenMission,
  onOpenSettings,
}: {
  project: Project | null;
  daemonConnected: boolean;
  onOpenMission(missionId: string): void;
  onOpenSettings(): void;
}) {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeConvId, setActiveConvId] = useState<string | null>(null);
  const [convDetail, setConvDetail] = useState<ConversationDetail | null>(null);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const [thinking, setThinking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<
    "no_project" | "loading" | "loaded" | "daemon_unavailable" | "no_chat"
  >(project ? "loading" : "no_project");
  const [renameTarget, setRenameTarget] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");

  // Security state
  const [secStatus, setSecStatus] = useState<SecurityStatus | null>(null);
  const [findings, setFindings] = useState<SecurityModeFinding[]>([]);
  const [attackPaths, setAttackPaths] = useState<SecurityAttackPath[]>([]);
  const [audit, setAudit] = useState<SecurityAuditResult | null>(null);
  const [auditDepth, setAuditDepth] = useState<
    "quick" | "full" | "cloud" | "ai" | "adversarial"
  >("quick");
  const [report, setReport] = useState<SecurityReportData | null>(null);
  const [selectedFinding, setSelectedFinding] = useState<SecurityModeFinding | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [showScopeForm, setShowScopeForm] = useState(false);
  const [scopeForm, setScopeForm] = useState({
    target: "",
    scope_kind: "repository",
    auth_state: "read-only",
    allowed_hosts: "",
    allowed_ports: "",
    allowed_techniques: "",
  });
  const [panelTab, setPanelTab] = useState<
    "scope" | "findings" | "paths" | "report" | "status"
  >("scope");
  const [expandedFinding, setExpandedFinding] = useState<string | null>(null);
  // G5-35 quality metrics + G5-26 credential lifecycle, loaded from the same
  // daemon control plane as every other security surface.
  const [metrics, setMetrics] = useState<SecurityQualityMetrics | null>(null);
  const [secretLifecycle, setSecretLifecycle] = useState<SecuritySecretLifecycle | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!project) {
      setStatus("no_project");
      setConversations([]);
      setActiveConvId(null);
      setConvDetail(null);
      return;
    }
    setStatus("loading");
    let cancelled = false;
    (async () => {
      const convs = await daemon.listConversations(project.path);
      if (cancelled) return;
      const securityConvs = convs.filter((c) => c.mode === "SECURITY");
      setConversations(securityConvs);
      if (!activeConvId && securityConvs.length > 0) {
        setActiveConvId(securityConvs[0].id);
      }
      setStatus(securityConvs.length > 0 ? "loaded" : "no_chat");
    })();
    return () => {
      cancelled = true;
    };
  }, [project]);

  useEffect(() => {
    if (!activeConvId) {
      setConvDetail(null);
      return;
    }
    let cancelled = false;
    (async () => {
      const detail = await daemon.getConversation(activeConvId);
      if (cancelled) return;
      if (detail) setConvDetail(detail);
    })();
    return () => {
      cancelled = true;
    };
  }, [activeConvId]);

  // Load security status/findings/paths for the active conversation
  useEffect(() => {
    if (!activeConvId) {
      setSecStatus(null);
      setFindings([]);
      setAttackPaths([]);
      setAudit(null);
      setReport(null);
      setSelectedFinding(null);
      setMetrics(null);
      setSecretLifecycle(null);
      return;
    }
    let cancelled = false;
    (async () => {
      const [s, f, p] = await Promise.all([
        daemon.securityStatus(activeConvId),
        daemon.securityFindings(activeConvId),
        daemon.securityAttackPaths(activeConvId),
      ]);
      if (cancelled) return;
      if (s.ok) setSecStatus(s.status ?? null);
      if (f.ok) setFindings(f.findings ?? []);
      if (p.ok) setAttackPaths(p.attackPaths ?? []);
    })();
    return () => {
      cancelled = true;
    };
  }, [activeConvId]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [convDetail?.messages?.length]);

  const refreshConversations = useCallback(async () => {
    if (!project) return;
    const convs = await daemon.listConversations(project.path);
    setConversations(convs.filter((c) => c.mode === "SECURITY"));
  }, [project]);

  const refreshActive = useCallback(async () => {
    if (!activeConvId) return;
    const detail = await daemon.getConversation(activeConvId);
    if (detail) setConvDetail(detail);
  }, [activeConvId]);

  const refreshSecurity = useCallback(async () => {
    if (!activeConvId) return;
    const [s, f, p, q] = await Promise.all([
      daemon.securityStatus(activeConvId),
      daemon.securityFindings(activeConvId),
      daemon.securityAttackPaths(activeConvId),
      daemon.securityQualityMetrics(activeConvId),
    ]);
    if (s.ok) setSecStatus(s.status ?? null);
    if (f.ok) setFindings(f.findings ?? []);
    if (p.ok) setAttackPaths(p.attackPaths ?? []);
    if (q.ok) setMetrics(q.metrics ?? null);
  }, [activeConvId]);

  const handleNewChat = async () => {
    if (!project) return;
    setError(null);
    const result = await daemon.createConversation(
      project.path,
      "SECURITY",
      "Security Audit"
    );
    if (result.ok && result.conversation_id) {
      setActiveConvId(result.conversation_id);
      await refreshConversations();
      setStatus("loaded");
    } else {
      setError(result.error || "Could not create security chat");
    }
  };

  const handleSend = async () => {
    const trimmed = input.trim();
    if (!trimmed || !activeConvId || sending || thinking) return;
    setSending(true);
    setError(null);
    const userResult = await daemon.appendMessage(activeConvId, "user", trimmed);
    if (!userResult.ok) {
      setError(userResult.error || "Could not send message");
      setSending(false);
      return;
    }
    setInput("");
    await refreshActive();
    setSending(false);
    setThinking(true);
    const replyResult = await daemon.securitySend(activeConvId, trimmed, []);
    if (replyResult.ok) {
      await refreshActive();
    } else {
      setError(replyResult.error || "Could not get response");
    }
    setThinking(false);
  };

  const handleSetScope = async () => {
    if (!activeConvId) return;
    setBusy("scope");
    setError(null);
    const result = await daemon.securitySetScope(
      activeConvId,
      scopeForm.target,
      scopeForm.scope_kind,
      scopeForm.auth_state,
      scopeForm.allowed_hosts
        .split(",")
        .map((s) => s.trim())
        .filter((s) => s.length > 0),
      scopeForm.allowed_ports
        .split(",")
        .map((s) => parseInt(s.trim(), 10))
        .filter((n) => !Number.isNaN(n) && n > 0 && n < 65536),
      scopeForm.allowed_techniques
        .split(",")
        .map((s) => s.trim())
        .filter((s) => s.length > 0)
    );
    if (result.ok) {
      setShowScopeForm(false);
      await refreshSecurity();
    } else {
      setError(result.error || "Could not set scope");
    }
    setBusy(null);
  };

  const handleAudit = async () => {
    if (!activeConvId) return;
    setBusy("audit");
    setError(null);
    const result = await daemon.securityAudit(activeConvId, auditDepth);
    if (result.ok) {
      setAudit(result.audit ?? null);
      await refreshSecurity();
    } else {
      setError(result.error || "Audit failed");
    }
    setBusy(null);
  };

  const handleFindingDetail = async (findingId: string) => {
    if (!activeConvId) return;
    setDetailLoading(true);
    setError(null);
    setSecretLifecycle(null);
    const result = await daemon.securityFindingDetail(activeConvId, findingId);
    if (result.ok) {
      setSelectedFinding(result.finding ?? null);
      setExpandedFinding(findingId);
      // Credential lifecycle (G5-26): secret-exposure findings carry a
      // rotation/revocation workflow that requires explicit human approval.
      if (result.finding?.category === "secret") {
        const lifecycle = await daemon.securitySecretLifecycle(activeConvId, findingId);
        if (lifecycle.ok) setSecretLifecycle(lifecycle.lifecycle ?? null);
      }
    } else {
      setError(result.error || "Could not load finding detail");
    }
    setDetailLoading(false);
  };

  const handleTransition = async (findingId: string, target: string) => {
    if (!activeConvId) return;
    setBusy(`transition:${findingId}`);
    setError(null);
    const result = await daemon.securityFindingTransition(
      activeConvId,
      findingId,
      target
    );
    if (result.ok) {
      await refreshSecurity();
      if (selectedFinding?.id === findingId) {
        setSelectedFinding(result.finding ?? null);
      }
    } else {
      setError(result.error || "Transition rejected");
    }
    setBusy(null);
  };

  const handleValidate = async (findingId: string) => {
    if (!activeConvId) return;
    setBusy(`validate:${findingId}`);
    setError(null);
    const result = await daemon.securityValidate(activeConvId, findingId);
    if (result.ok) {
      await refreshSecurity();
      await handleFindingDetail(findingId);
    } else {
      setError(result.error || "Validation blocked");
    }
    setBusy(null);
  };

  const handleRemediate = async (findingId: string) => {
    if (!activeConvId) return;
    if (!window.confirm("Approve remediation for this confirmed finding?\n\nThis creates a repair mission through the normal Kernel/Tool Broker path.")) {
      return;
    }
    setBusy(`remediate:${findingId}`);
    setError(null);
    const result = await daemon.securityRemediate(activeConvId, findingId, true);
    if (result.ok) {
      if (result.remediation?.mission_id) {
        onOpenMission(result.remediation.mission_id);
      }
      await refreshSecurity();
      await refreshActive();
    } else {
      setError(result.error || "Remediation not approved");
    }
    setBusy(null);
  };

  const handleRetest = async () => {
    if (!activeConvId) return;
    setBusy("retest");
    setError(null);
    const result = await daemon.securityRetest(activeConvId);
    if (result.ok) {
      await refreshSecurity();
    } else {
      setError(result.error || "Retest failed");
    }
    setBusy(null);
  };

  const handleReport = async () => {
    if (!activeConvId) return;
    setBusy("report");
    setError(null);
    const result = await daemon.securityReport(activeConvId);
    if (result.ok) {
      setReport(result.report ?? null);
      setPanelTab("report");
      await refreshSecurity();
    } else {
      setError(result.error || "Could not generate report");
    }
    setBusy(null);
  };

  const handleRename = async (id: string) => {
    const trimmed = renameValue.trim();
    if (!trimmed) {
      setRenameTarget(null);
      return;
    }
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

  function renderMessage(msg: Message) {
    const isUser = msg.role === "user";
    const isAssistant = msg.role === "assistant";
    let providerModel: { provider_id?: string; model_name?: string } | null = null;
    if (isAssistant && msg.metadata) {
      try {
        const meta =
          typeof msg.metadata === "string"
            ? JSON.parse(msg.metadata)
            : msg.metadata;
        if (meta?.provider_model) providerModel = meta.provider_model;
      } catch {
        providerModel = null;
      }
    }
    return (
      <div key={msg.id} className={`mb-4 flex ${isUser ? "justify-end" : "justify-start"}`}>
        <div className="max-w-[85%]">
          <div
            className={`rounded-2xl px-4 py-3 ${
              isUser
                ? "bg-primary text-on-primary rounded-br-md"
                : "neo-pressed text-on-surface rounded-bl-md"
            }`}
          >
            <p className="text-sm whitespace-pre-wrap break-words">{msg.content}</p>
            {isAssistant && (providerModel || msg.metadata) && (
              <div className="mt-2 flex items-center gap-2 text-[10px] text-on-surface-variant">
                <Icon name="security" size={12} />
                <span>
                  {providerModel?.model_name
                    ? providerModel.model_name
                    : providerModel?.provider_id
                      ? providerModel.provider_id
                      : "Security assistant"}
                </span>
              </div>
            )}
            {msg.mission_ref && (
              <div className="mt-2 flex items-center gap-2 text-xs text-on-surface-variant">
                <Icon name="terminal" size={12} />
                <span>Repair mission: {msg.mission_ref}</span>
                <button onClick={() => onOpenMission(msg.mission_ref!)} className="underline hover:opacity-80">
                  View
                </button>
              </div>
            )}
          </div>
          <p className="text-[10px] text-on-surface-variant mt-0.5 px-1">
            {formatTime(msg.created_at_ms)}
          </p>
        </div>
      </div>
    );
  }

  function renderFindingRow(finding: SecurityModeFinding) {
    const expanded = expandedFinding === finding.id;
    const sevColor = SEVERITY_COLORS[finding.severity] ?? SEVERITY_COLORS.Low;
    const stateColor = STATE_COLORS[finding.state] ?? STATE_COLORS.New;
    return (
      <div key={finding.id} className="rounded-xl border border-outline-variant/30 bg-background">
        <button
          onClick={() => handleFindingDetail(finding.id)}
          className="w-full text-left px-3 py-2.5 hover:bg-surface-variant/20 transition-colors"
        >
          <div className="flex items-center gap-2 flex-wrap">
            <span className={`text-[10px] px-2 py-0.5 rounded-full font-medium ${sevColor}`}>
              {finding.severity}
            </span>
            <span className={`text-[10px] px-2 py-0.5 rounded-full font-medium ${stateColor}`}>
              {finding.state}
            </span>
            <span className="text-xs text-on-surface-variant">
              confidence {finding.confidence}% · exploitability {finding.exploitability}%
            </span>
          </div>
          <p className="text-sm font-medium text-on-surface mt-1 break-words">{finding.root_cause}</p>
          <p className="text-[11px] text-on-surface-variant mt-0.5 truncate">{finding.affected_code || finding.affected_asset}</p>
        </button>
        {expanded && selectedFinding && selectedFinding.id === finding.id && (
          <div className="px-3 pb-3 border-t border-outline-variant/30 pt-2 space-y-2">
            <p className="text-xs text-on-surface-variant">
              <span className="text-on-surface font-medium">Category:</span> {selectedFinding.category}
            </p>
            <p className="text-xs text-on-surface-variant">
              <span className="text-on-surface font-medium">Why it matters:</span>{" "}
              {selectedFinding.affected_code} exposed in environment {selectedFinding.environment}
            </p>
            <p className="text-xs text-on-surface-variant">
              <span className="text-on-surface font-medium">Source commit:</span>{" "}
              <span className="font-mono">{selectedFinding.source_commit}</span>
            </p>
            {selectedFinding.evidence_refs && (
              <p className="text-xs text-on-surface-variant">
                <span className="text-on-surface font-medium">Evidence refs:</span>{" "}
                <span className="font-mono text-[10px]">{selectedFinding.evidence_refs}</span>
              </p>
            )}
            {selectedFinding.remediation && (
              <p className="text-xs text-on-surface-variant">
                <span className="text-on-surface font-medium">Remediation:</span>{" "}
                {selectedFinding.remediation}
              </p>
            )}
            {selectedFinding.mission_ref && (
              <button
                onClick={() => onOpenMission(selectedFinding.mission_ref!)}
                className="text-xs text-primary underline"
              >
                View repair mission {selectedFinding.mission_ref}
              </button>
            )}
            {selectedFinding.validations && selectedFinding.validations.length > 0 && (
              <div className="text-xs text-on-surface-variant">
                <span className="text-on-surface font-medium">Safe validations:</span>
                <ul className="list-disc list-inside ml-2">
                  {selectedFinding.validations.map((v: SecurityValidation) => (
                    <li key={v.id}>
                      {v.state} — {v.detail} (evidence {v.evidence_ref})
                    </li>
                  ))}
                </ul>
              </div>
            )}
            {selectedFinding.regressions && selectedFinding.regressions.length > 0 && (
              <p className="text-xs text-on-surface-variant">
                <span className="text-on-surface font-medium">Regression protection:</span>{" "}
                {selectedFinding.regressions.map((r) => `${r.regression_type}:${r.state}`).join(", ")}
              </p>
            )}
            {selectedFinding.attack_paths && selectedFinding.attack_paths.length > 0 && (
              <div className="rounded-lg bg-red-500/5 px-3 py-2 space-y-1">
                <p className="text-xs text-on-surface font-medium">
                  Attack paths traversing this finding
                </p>
                {selectedFinding.attack_paths.map((path) => (
                  <div key={path.id} className="text-[11px] text-on-surface-variant">
                    <p>
                      <span className="text-on-surface">{path.entry_point ?? "entry"}</span>
                      {path.privilege_required && ` (${path.privilege_required} required)`}
                      {" → impact: "}
                      <span className="text-red-600 dark:text-red-400">{path.impact ?? "unknown"}</span>
                    </p>
                    {(path.steps ?? []).length > 0 && (
                      <ol className="list-decimal list-inside ml-2">
                        {(path.steps ?? []).map((step, i) => (
                          <li key={i}>
                            {step.label ?? step.step_kind ?? "step"}
                          </li>
                        ))}
                      </ol>
                    )}
                    <p className="text-[10px]">
                      validation: {path.validation_state ?? "unvalidated"} · assets:{" "}
                      {(path.affected_assets ?? []).join(", ") || "—"}
                    </p>
                  </div>
                ))}
              </div>
            )}
            {selectedFinding.category === "secret" && secretLifecycle && (
              <div className="rounded-lg bg-amber-500/5 px-3 py-2 space-y-1">
                <p className="text-xs text-on-surface font-medium">
                  Credential lifecycle — removal alone is not closure
                </p>
                <ol className="list-decimal list-inside ml-1 text-xs text-on-surface-variant">
                  {secretLifecycle.steps.map((step) => (
                    <li key={step.step} className={step.requires_human_approval ? "text-amber-600 dark:text-amber-400" : ""}>
                      {step.step.replace(/_/g, " ").toLowerCase()}
                      {step.requires_human_approval && " (requires your approval)"}
                    </li>
                  ))}
                </ol>
                <p className="text-[10px] text-on-surface-variant">{secretLifecycle.note}</p>
              </div>
            )}
            <div className="flex flex-wrap gap-2 pt-1">
              {["Triaged", "Validating", "Dismissed", "NeedsManualReview"].map((target) => (
                <button
                  key={target}
                  disabled={busy === `transition:${finding.id}`}
                  onClick={() => handleTransition(finding.id, target)}
                  className="text-[11px] px-2.5 py-1 rounded-lg neo-button text-on-surface-variant hover:text-primary disabled:opacity-50"
                >
                  → {target}
                </button>
              ))}
              <button
                disabled={busy === `validate:${finding.id}` || !secStatus?.active_testing_allowed}
                onClick={() => handleValidate(finding.id)}
                title={
                  secStatus?.active_testing_allowed
                    ? "Run safe canary validation"
                    : "Active validation requires an authorized active scope"
                }
                className="text-[11px] px-2.5 py-1 rounded-lg neo-button text-emerald-600 dark:text-emerald-400 disabled:opacity-50"
              >
                Validate
              </button>
              <button
                disabled={busy === `remediate:${finding.id}` || finding.state !== "Confirmed"}
                onClick={() => handleRemediate(finding.id)}
                title={
                  finding.state === "Confirmed"
                    ? "Approve and create repair mission"
                    : "Only CONFIRMED findings may be remediated"
                }
                className="text-[11px] px-2.5 py-1 rounded-lg neo-button text-red-600 dark:text-red-400 disabled:opacity-50"
              >
                Remediate
              </button>
            </div>
          </div>
        )}
      </div>
    );
  }

  function renderAttackPath(path: SecurityAttackPath) {
    return (
      <div key={path.id} className="rounded-xl border border-outline-variant/30 bg-background p-3">
        <div className="flex items-center gap-2 mb-2">
          <span className="text-[10px] px-2 py-0.5 rounded-full bg-red-500/10 text-red-600 dark:text-red-400 font-medium">
            {path.impact}
          </span>
          <span className="text-xs text-on-surface-variant">privilege: {path.privilege_required}</span>
        </div>
        <div className="flex flex-col gap-1">
          {path.steps.map((step, index) => (
            <div key={index} className="flex items-center gap-2">
              <span className="text-[10px] text-on-surface-variant w-20 shrink-0">{step.step_kind}</span>
              <span className="text-xs text-on-surface flex-1 break-words">{step.label}</span>
              {index < path.steps.length - 1 && (
                <Icon name="arrow_downward" size={12} className="text-on-surface-variant/50 shrink-0" />
              )}
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (status === "no_project") {
    return (
      <main className="flex-1 flex items-center justify-center">
        <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
          <Icon name="security" size={40} className="text-primary mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-on-surface mb-2">Open a Project</h3>
          <p className="text-sm text-on-surface-variant">
            Select a project to run a governed security audit workspace.
          </p>
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 flex overflow-hidden">
      {/* Conversation List */}
      <aside className="w-56 shrink-0 flex flex-col border-r border-outline-variant/40 dark:border-white/5 bg-surface/50">
        <div className="p-3 border-b border-outline-variant/40 dark:border-white/5">
          <button
            onClick={handleNewChat}
            className="w-full neo-button rounded-xl py-2.5 flex items-center justify-center gap-2 text-sm font-medium text-primary"
          >
            <Icon name="add" size={18} />
            New Security Chat
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
                  <p className="text-sm font-medium truncate mt-0.5">{conv.title}</p>
                )}
                <p className="text-[10px] text-on-surface-variant mt-0.5">{formatTime(conv.updated_at_ms)}</p>
              </button>
              <div className="flex gap-1 px-3 mb-1">
                <button
                  onClick={() => {
                    setRenameTarget(conv.id);
                    setRenameValue(conv.title);
                  }}
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
              No security audits yet. Start a new security chat.
            </div>
          )}
        </div>
      </aside>

      {/* Chat Area */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {!activeConvId ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
              <Icon name="security" size={40} className="text-primary mx-auto mb-4" />
              <h3 className="text-lg font-semibold text-on-surface mb-2">Select a Security Chat</h3>
              <p className="text-sm text-on-surface-variant">
                Choose a security conversation or start a new audit.
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
                  <span className="text-xs font-bold uppercase text-emerald-600 dark:text-emerald-400">
                    Security Mode
                  </span>
                )}
              </div>
              <div className="flex items-center gap-3">
                {secStatus?.final_status && (
                  <span className="text-[10px] px-2 py-1 rounded-full bg-on-surface-variant/10 text-on-surface-variant font-medium">
                    {secStatus.final_status}
                  </span>
                )}
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
            </div>

            <div className="flex-1 flex overflow-hidden">
              {/* Messages */}
              <div className="flex-1 overflow-y-auto px-6 py-4">
                {convDetail?.messages?.map(renderMessage)}
                {thinking && (
                  <div className="flex justify-start mb-4">
                    <div className="neo-pressed rounded-2xl px-4 py-3 rounded-bl-md">
                      <div className="flex items-center gap-2 text-sm text-on-surface-variant">
                        <Icon name="autorenew" size={16} className="animate-spin" />
                        <span>Analyzing...</span>
                      </div>
                    </div>
                  </div>
                )}
                {(!convDetail || convDetail.messages.length === 0) && !thinking && (
                  <div className="text-center text-xs text-on-surface-variant py-10">
                    Set a security scope, run an audit, and review findings — or ask about this
                    project's security posture.
                  </div>
                )}
                <div ref={messagesEndRef} />
              </div>

              {/* Security Panel */}
              <aside className="w-80 shrink-0 border-l border-outline-variant/40 dark:border-white/5 flex flex-col bg-surface/30">
                <div className="flex border-b border-outline-variant/40 dark:border-white/5">
                  {(["scope", "findings", "paths", "report", "status"] as const).map((tab) => (
                    <button
                      key={tab}
                      onClick={() => setPanelTab(tab)}
                      className={`flex-1 py-2 text-[11px] font-medium transition-colors ${
                        panelTab === tab
                          ? "text-emerald-600 dark:text-emerald-400 border-b-2 border-emerald-500"
                          : "text-on-surface-variant hover:text-on-surface"
                      }`}
                    >
                      {tab === "scope" ? "Scope" : tab === "paths" ? "Attack Paths" : tab.charAt(0).toUpperCase() + tab.slice(1)}
                    </button>
                  ))}
                </div>

                <div className="flex-1 overflow-y-auto p-3 space-y-3">
                  {error && (
                    <div className="flex items-center gap-2 text-xs text-red-600 dark:text-red-400 bg-red-500/5 rounded-lg px-3 py-2">
                      <Icon name="error" size={14} fill /> {error}
                    </div>
                  )}

                  {panelTab === "scope" && (
                    <div className="space-y-3">
                      {secStatus?.scope ? (
                        <div className="rounded-xl border border-outline-variant/30 bg-background p-3 space-y-2">
                          <div className="flex items-center justify-between">
                            <span className="text-sm font-semibold text-on-surface">Security Scope</span>
                            <span className={`text-[10px] px-2 py-0.5 rounded-full font-medium ${
                              secStatus.active_testing_allowed
                                ? "bg-emerald-500/10 text-emerald-600"
                                : "bg-amber-500/10 text-amber-600"
                            }`}>
                              {secStatus.active_testing_allowed ? "Active Testing" : "Read-Only"}
                            </span>
                          </div>
                          <p className="text-xs text-on-surface-variant break-words">Target: {secStatus.scope.target}</p>
                          <p className="text-xs text-on-surface-variant">
                            Kind: {SCOPE_KIND_LABEL[secStatus.scope.kind] ?? secStatus.scope.kind}
                          </p>
                          <p className="text-xs text-on-surface-variant">
                            Authorization: {AUTH_LABEL[secStatus.scope.authorization] ?? secStatus.scope.authorization}
                          </p>
                          {secStatus.scope.allowed_hosts.length > 0 && (
                            <p className="text-xs text-on-surface-variant">
                              Hosts: {secStatus.scope.allowed_hosts.join(", ")}
                            </p>
                          )}
                          {secStatus.scope.allowed_ports.length > 0 && (
                            <p className="text-xs text-on-surface-variant">
                              Ports: {secStatus.scope.allowed_ports.join(", ")}
                            </p>
                          )}
                          <div className="flex flex-wrap items-center gap-2 pt-1">
                            <button
                              onClick={() => setShowScopeForm(true)}
                              className="text-[11px] px-2.5 py-1 rounded-lg neo-button text-on-surface-variant hover:text-primary"
                            >
                              Change scope
                            </button>
                            <select
                              value={auditDepth}
                              onChange={(e) =>
                                setAuditDepth(e.target.value as typeof auditDepth)
                              }
                              className="text-[11px] px-2 py-1 rounded-lg neo-input bg-background text-on-surface"
                              aria-label="Audit depth"
                              title="Scanner set per depth: quick = fast content scanners; full adds dependency/IaC; cloud adds IaC emphasis; adversarial adds ZAP DAST against an authorized localhost target"
                            >
                              <option value="quick">Quick</option>
                              <option value="full">Full</option>
                              <option value="cloud">Cloud</option>
                              <option value="ai">AI</option>
                              <option value="adversarial">Adversarial (DAST)</option>
                            </select>
                            <button
                              disabled={busy === "audit"}
                              onClick={handleAudit}
                              className="text-[11px] px-2.5 py-1 rounded-lg neo-button text-primary disabled:opacity-50"
                            >
                              {busy === "audit" ? "Auditing..." : "Run Audit"}
                            </button>
                          </div>
                        </div>
                      ) : (
                        <div className="rounded-xl border border-outline-variant/30 bg-background p-3 space-y-2">
                          <p className="text-sm font-semibold text-on-surface">Establish Scope</p>
                          <p className="text-xs text-on-surface-variant">
                            Active testing never begins without an explicit scope classification.
                            Repository-only scope is read-only audit.
                          </p>
                          <button
                            onClick={() => setShowScopeForm(true)}
                            className="w-full text-[11px] px-2.5 py-1.5 rounded-lg neo-button text-primary"
                          >
                            Set Scope
                          </button>
                        </div>
                      )}
                    </div>
                  )}

                  {panelTab === "findings" && (
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <span className="text-sm font-semibold text-on-surface">
                          Findings ({findings.length})
                        </span>
                        <div className="flex gap-1.5">
                          <button
                            disabled={busy === "retest"}
                            onClick={handleRetest}
                            className="text-[11px] px-2 py-1 rounded-lg neo-button text-on-surface-variant hover:text-primary disabled:opacity-50"
                          >
                            Retest
                          </button>
                        </div>
                      </div>
                      {findings.length === 0 && (
                        <p className="text-xs text-on-surface-variant py-6 text-center">
                          Run an audit to produce normalized findings.
                        </p>
                      )}
                      {findings.map(renderFindingRow)}
                    </div>
                  )}

                  {panelTab === "paths" && (
                    <div className="space-y-2">
                      <span className="text-sm font-semibold text-on-surface">
                        Attack Paths ({attackPaths.length})
                      </span>
                      {attackPaths.length === 0 && (
                        <p className="text-xs text-on-surface-variant py-6 text-center">
                          Attack paths are composed after an audit finds evidence.
                        </p>
                      )}
                      {attackPaths.map(renderAttackPath)}
                    </div>
                  )}

                  {panelTab === "report" && (
                    <div className="space-y-2">
                      <button
                        disabled={busy === "report"}
                        onClick={handleReport}
                        className="w-full text-[11px] px-2.5 py-1.5 rounded-lg neo-button text-primary disabled:opacity-50"
                      >
                        {busy === "report" ? "Generating..." : "Generate Security Report"}
                      </button>
                      {report ? (
                        <div className="rounded-xl border border-outline-variant/30 bg-background p-3 space-y-2">
                          <p className="text-sm font-semibold text-on-surface">{report.final_status}</p>
                          <div className="text-xs text-on-surface-variant space-y-1">
                            {report.differential.summary.map((line, i) => (
                              <p key={i}>- {line}</p>
                            ))}
                          </div>
                          <pre className="text-[10px] text-on-surface-variant whitespace-pre-wrap max-h-64 overflow-y-auto bg-surface/60 rounded-lg p-2">
                            {report.markdown}
                          </pre>
                        </div>
                      ) : (
                        <p className="text-xs text-on-surface-variant py-6 text-center">
                          The report includes executive summary, scope, findings, attack paths,
                          validations, differential review and final status.
                        </p>
                      )}
                    </div>
                  )}

                  {panelTab === "status" && (
                    <div className="space-y-2">
                      <span className="text-sm font-semibold text-on-surface">Audit State</span>
                      {secStatus ? (
                        <div className="rounded-xl border border-outline-variant/30 bg-background p-3 space-y-1.5 text-xs text-on-surface-variant">
                          <p>Audit status: <span className="text-on-surface">{secStatus.audit_status}</span></p>
                          <p>Final status: <span className="text-on-surface">{secStatus.final_status}</span></p>
                          <p className="font-mono text-[10px]">commit: {secStatus.source_commit}</p>
                          <p>Findings: {secStatus.findings_total} · Confirmed: {secStatus.findings_confirmed}</p>
                          <p>Attack paths: {secStatus.attack_path_count} · Validations: {secStatus.validation_count}</p>
                          <p>Regression protections: {secStatus.regression_count}</p>
                          {secStatus.scanners_unavailable > 0 && (
                            <p className="text-amber-600 dark:text-amber-400">
                              Scanners unavailable in last audit: {secStatus.scanners_unavailable}
                            </p>
                          )}
                          {Object.entries(secStatus.state_counts ?? {}).map(([state, count]) => (
                            <p key={state}>
                              {state}: <span className="text-on-surface">{count}</span>
                            </p>
                          ))}
                          {audit && (
                            <div className="pt-1 border-t border-outline-variant/30 space-y-1">
                              <p>
                                Last audit depth:{" "}
                                <span className="text-on-surface">
                                  {audit.audit_depth ?? "quick"}
                                </span>
                              </p>
                              <p>
                                AI security surface:{" "}
                                <span className="text-on-surface">
                                  {audit.ai_security_applicable ? "detected" : "not applicable"}
                                </span>
                              </p>
                              {audit.scanner_availability?.map((scanner) => (
                                <p key={scanner.adapter}>
                                  {scanner.adapter}:{" "}
                                  <span
                                    className={
                                      scanner.availability === "Available"
                                        ? "text-emerald-600 dark:text-emerald-400"
                                        : "text-amber-600 dark:text-amber-400"
                                    }
                                  >
                                    {scanner.availability}
                                    {scanner.version ? ` (${scanner.version})` : ""}
                                  </span>
                                </p>
                              ))}
                            </div>
                          )}
                          {metrics && (
                            <div className="pt-1 border-t border-outline-variant/30 space-y-1">
                              <p className="text-on-surface font-medium">Quality</p>
                              <p>
                                Confirmed rate:{" "}
                                <span className="text-on-surface">
                                  {(metrics.confirmed_rate * 100).toFixed(0)}%
                                </span>
                              </p>
                              <p>
                                False-positive dismissal rate:{" "}
                                <span className="text-on-surface">
                                  {(metrics.false_positive_dismissal_rate * 100).toFixed(0)}%
                                </span>
                              </p>
                              <p>
                                Canary proofs: <span className="text-on-surface">{metrics.validation_success}</span>
                                {" · "}
                                Blocked attempts:{" "}
                                <span className="text-on-surface">{metrics.validation_blocked}</span>
                              </p>
                              <p>
                                Regression protections:{" "}
                                <span className="text-on-surface">{metrics.regression_protections_active}</span>
                                {metrics.regression_protections_broken > 0 && (
                                  <span className="text-red-600 dark:text-red-400">
                                    {" "}· {metrics.regression_protections_broken} broken
                                  </span>
                                )}
                              </p>
                            </div>
                          )}
                        </div>
                      ) : (
                        <p className="text-xs text-on-surface-variant py-6 text-center">
                          No security scope established yet.
                        </p>
                      )}
                    </div>
                  )}
                </div>
              </aside>
            </div>

            {/* Composer */}
            <div className="shrink-0 px-6 py-4 border-t border-outline-variant/40 dark:border-white/5">
              <div className="neo-raised rounded-[20px] p-2">
                <div className="neo-pressed rounded-[16px] px-4 py-3 flex items-end gap-2">
                  <textarea
                    ref={inputRef}
                    className="flex-1 bg-transparent border-none outline-none resize-none text-sm text-on-surface placeholder:text-on-surface-variant/50 max-h-[120px] focus:ring-0 p-0"
                    placeholder="Ask about the project's security posture, or request an audit..."
                    value={input}
                    disabled={sending || thinking}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                    rows={1}
                  />
                  <button
                    onClick={handleSend}
                    disabled={sending || thinking || !input.trim()}
                    className="w-9 h-9 rounded-full bg-primary flex items-center justify-center text-on-primary disabled:opacity-50 active:scale-95 transition-all"
                    aria-label="Send message"
                  >
                    <Icon name={sending || thinking ? "autorenew" : "send"} size={18} />
                  </button>
                </div>
              </div>
              <div className="flex justify-end mt-2">
                <button
                  onClick={onOpenSettings}
                  className="neo-button rounded-xl px-4 py-2 text-sm font-medium text-on-surface-variant flex items-center gap-2"
                >
                  <Icon name="settings" size={16} />
                  Settings
                </button>
              </div>
            </div>
          </>
        )}
      </div>

      {/* Scope form modal */}
      {showScopeForm && (
        <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={() => setShowScopeForm(false)}>
          <div
            className="bg-surface rounded-2xl p-6 max-w-md w-full mx-4 neo-raised max-h-[80vh] overflow-y-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-semibold text-on-surface mb-1">Establish Security Scope</h3>
            <p className="text-xs text-on-surface-variant mb-4">
              Active testing is blocked unless the scope classification authorizes it. Production
              read-only always blocks active effects.
            </p>
            <div className="space-y-3">
              <div>
                <label className="text-xs text-on-surface-variant mb-1 block">Target</label>
                <input
                  className="neo-input rounded-lg px-3 py-2 text-sm w-full"
                  placeholder="e.g. repository path or http://127.0.0.1:8080"
                  value={scopeForm.target}
                  onChange={(e) => setScopeForm({ ...scopeForm, target: e.target.value })}
                />
              </div>
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <label className="text-xs text-on-surface-variant mb-1 block">Scope kind</label>
                  <select
                    className="neo-input rounded-lg px-3 py-2 text-sm w-full"
                    value={scopeForm.scope_kind}
                    onChange={(e) => setScopeForm({ ...scopeForm, scope_kind: e.target.value })}
                  >
                    <option value="repository">Repository Only</option>
                    <option value="local">Local Only</option>
                    <option value="staging">Staging Authorized</option>
                    <option value="production-read-only">Production Read-Only</option>
                    <option value="production-active-approved">Production Active (Approved)</option>
                    <option value="cloud-lab">Cloud Lab Authorized</option>
                  </select>
                </div>
                <div>
                  <label className="text-xs text-on-surface-variant mb-1 block">Authorization</label>
                  <select
                    className="neo-input rounded-lg px-3 py-2 text-sm w-full"
                    value={scopeForm.auth_state}
                    onChange={(e) => setScopeForm({ ...scopeForm, auth_state: e.target.value })}
                  >
                    <option value="read-only">Read-Only Audit</option>
                    <option value="active">Active Validation</option>
                    <option value="adversarial">Authorized Adversarial</option>
                  </select>
                </div>
              </div>
              <div>
                <label className="text-xs text-on-surface-variant mb-1 block">
                  Allowed hosts (comma separated)
                </label>
                <input
                  className="neo-input rounded-lg px-3 py-2 text-sm w-full"
                  placeholder="e.g. 127.0.0.1, fixture.local"
                  value={scopeForm.allowed_hosts}
                  onChange={(e) => setScopeForm({ ...scopeForm, allowed_hosts: e.target.value })}
                />
              </div>
              <div>
                <label className="text-xs text-on-surface-variant mb-1 block">
                  Allowed ports (comma separated)
                </label>
                <input
                  className="neo-input rounded-lg px-3 py-2 text-sm w-full"
                  placeholder="e.g. 8080, 443"
                  value={scopeForm.allowed_ports}
                  onChange={(e) => setScopeForm({ ...scopeForm, allowed_ports: e.target.value })}
                />
              </div>
              <div>
                <label className="text-xs text-on-surface-variant mb-1 block">
                  Allowed techniques (comma separated)
                </label>
                <input
                  className="neo-input rounded-lg px-3 py-2 text-sm w-full"
                  placeholder="e.g. static, dependency, dast"
                  value={scopeForm.allowed_techniques}
                  onChange={(e) => setScopeForm({ ...scopeForm, allowed_techniques: e.target.value })}
                />
              </div>
              {busy === "scope" && (
                <p className="text-xs text-on-surface-variant">Setting scope...</p>
              )}
              <div className="flex gap-3 mt-2">
                <button
                  onClick={handleSetScope}
                  disabled={busy === "scope"}
                  className="neo-button px-5 py-2.5 rounded-xl text-sm text-primary font-medium flex items-center gap-2 disabled:opacity-50"
                >
                  <Icon name="verified_user" size={16} />
                  Set Scope
                </button>
                <button
                  onClick={() => setShowScopeForm(false)}
                  className="neo-button px-5 py-2.5 rounded-xl text-sm text-on-surface-variant font-medium"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
      {detailLoading && (
        <div className="fixed bottom-24 right-8 neo-raised rounded-xl px-4 py-2 text-xs text-on-surface-variant">
          Loading finding detail...
        </div>
      )}
    </main>
  );
}
