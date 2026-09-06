import { useEffect, useState, useRef, useCallback } from "react";
import type { KeyboardEvent } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { Project } from "./ProjectContext";
import type {
  Conversation,
  ConversationDetail,
  Message,
  Attachment,
  DiscussPlan,
  DiscussDecisionRecord,
  DiscussMessageMetadata,
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

export function DiscussView({
  project,
  daemonConnected,
  onOpenMission,
}: {
  project: Project | null;
  daemonConnected: boolean;
  onOpenMission(missionId: string): void;
}) {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeConvId, setActiveConvId] = useState<string | null>(null);
  const [convDetail, setConvDetail] = useState<ConversationDetail | null>(null);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const [thinking, setThinking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pendingAttachments, setPendingAttachments] = useState<Attachment[]>([]);
  const [status, setStatus] = useState<
    "no_project" | "loading" | "loaded" | "daemon_unavailable" | "no_chat"
  >(project ? "loading" : "no_project");
  const [renameTarget, setRenameTarget] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [plan, setPlan] = useState<DiscussPlan | null>(null);
  const [decisions, setDecisions] = useState<DiscussDecisionRecord[]>([]);
  const [planBusy, setPlanBusy] = useState(false);
  const [decisionDraft, setDecisionDraft] = useState<{
    messageId: string;
    excerpt: string;
  } | null>(null);
  const [decisionText, setDecisionText] = useState("");
  const [decisionRationale, setDecisionRationale] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Load conversations when project changes
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
      const discussConvs = convs.filter((c) => c.mode === "DISCUSS");
      setConversations(discussConvs);
      if (!activeConvId && discussConvs.length > 0) {
        setActiveConvId(discussConvs[0].id);
      }
      setStatus(discussConvs.length > 0 ? "loaded" : "no_chat");
    })();
    return () => {
      cancelled = true;
    };
  }, [project]);

  // Load conversation detail when active conversation changes
  useEffect(() => {
    if (!activeConvId) {
      setConvDetail(null);
      setPlan(null);
      setDecisions([]);
      setDecisionDraft(null);
      return;
    }
    let cancelled = false;
    (async () => {
      const detail = await daemon.getConversation(activeConvId);
      if (cancelled) return;
      if (detail) setConvDetail(detail);
      const planRes = await daemon.discussGetPlan(activeConvId);
      if (cancelled) return;
      if (planRes.ok && planRes.plan) {
        try {
          setPlan(JSON.parse(planRes.plan) as DiscussPlan);
        } catch {
          setPlan(null);
        }
      } else {
        setPlan(null);
      }
      const decRes = await daemon.discussGetDecisions(activeConvId);
      if (cancelled) return;
      if (decRes.ok && decRes.decisions) {
        try {
          const parsed = JSON.parse(decRes.decisions) as {
            decisions?: DiscussDecisionRecord[];
          };
          setDecisions(parsed.decisions ?? []);
        } catch {
          setDecisions([]);
        }
      } else {
        setDecisions([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [activeConvId]);

  // Scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [convDetail?.messages?.length]);

  const refreshConversations = useCallback(async () => {
    if (!project) return;
    const convs = await daemon.listConversations(project.path);
    setConversations(convs.filter((c) => c.mode === "DISCUSS"));
  }, [project]);

  const refreshActive = useCallback(async () => {
    if (!activeConvId) return;
    const detail = await daemon.getConversation(activeConvId);
    if (detail) setConvDetail(detail);
    // Load the structured plan + decisions for this conversation (if any).
    const planRes = await daemon.discussGetPlan(activeConvId);
    if (planRes.ok && planRes.plan) {
      try {
        setPlan(JSON.parse(planRes.plan) as DiscussPlan);
      } catch {
        setPlan(null);
      }
    } else {
      setPlan(null);
    }
    const decRes = await daemon.discussGetDecisions(activeConvId);
    if (decRes.ok && decRes.decisions) {
      try {
        const parsed = JSON.parse(decRes.decisions) as {
          decisions?: DiscussDecisionRecord[];
        };
        setDecisions(parsed.decisions ?? []);
      } catch {
        setDecisions([]);
      }
    } else {
      setDecisions([]);
    }
  }, [activeConvId]);

  const handleTurnIntoPlan = async () => {
    if (!activeConvId || planBusy) return;
    setPlanBusy(true);
    setError(null);
    const result = await daemon.discussTurnIntoPlan(activeConvId);
    if (result.ok && result.plan) {
      setPlan(result.plan);
      await refreshActive();
    } else {
      setError(result.error || "Could not create plan");
    }
    setPlanBusy(false);
  };

  const handleExecutePlan = async () => {
    if (!activeConvId || planBusy || !plan) return;
    setPlanBusy(true);
    setError(null);
    const result = await daemon.discussExecutePlan(activeConvId);
    if (result.ok && result.missionId) {
      await refreshActive();
      onOpenMission(result.missionId);
    } else {
      setError(result.error || "Could not execute plan");
    }
    setPlanBusy(false);
  };

  const handleAcceptDecision = async () => {
    if (!activeConvId || !decisionDraft || planBusy) return;
    if (!decisionText.trim()) {
      setError("Decision text is required");
      return;
    }
    setPlanBusy(true);
    setError(null);
    const result = await daemon.discussAcceptDecision(
      activeConvId,
      decisionDraft.messageId,
      decisionText.trim(),
      decisionRationale.trim()
    );
    if (result.ok) {
      setDecisionDraft(null);
      setDecisionText("");
      setDecisionRationale("");
      await refreshActive();
    } else {
      setError(result.error || "Could not accept decision");
    }
    setPlanBusy(false);
  };

  const handleNewChat = async () => {
    if (!project) return;
    setError(null);
    const result = await daemon.createConversation(
      project.path,
      "DISCUSS",
      "New Discussion"
    );
    if (result.ok && result.conversation_id) {
      setActiveConvId(result.conversation_id);
      await refreshConversations();
      setStatus("loaded");
    } else {
      setError(result.error || "Could not create discussion");
    }
  };

  const handleSend = async () => {
    const trimmed = input.trim();
    if (!trimmed || !activeConvId || sending || thinking) return;
    setSending(true);
    setError(null);
    // Append user message optimistically
    const userResult = await daemon.appendMessage(
      activeConvId,
      "user",
      trimmed
    );
    if (!userResult.ok) {
      setError(userResult.error || "Could not send message");
      setSending(false);
      return;
    }
    setInput("");
    await refreshActive();
    setSending(false);
    setThinking(true);
    // Send to backend for AI response
    const replyResult = await daemon.discussSend(
      activeConvId,
      trimmed,
      pendingAttachments.map((a) => a.id)
    );
    if (replyResult.ok) {
      setPendingAttachments([]);
      await refreshActive();
    } else {
      setError(replyResult.error || "Could not get response");
    }
    setThinking(false);
  };

  const handleAttach = async () => {
    if (!project || !activeConvId) return;
    setError(null);
    const result = await daemon.addAttachment(activeConvId, project.path);
    if (result.ok && result.attachment) {
      setPendingAttachments((prev) => [...prev, result.attachment!]);
    } else {
      setError(result.error || "Could not attach file");
    }
  };

  const handleRemovePending = (id: string) => {
    setPendingAttachments((prev) => prev.filter((a) => a.id !== id));
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
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

  // ── Render ────────────────────────────────────────────────────────────────

  function renderMessage(msg: Message) {
    const isUser = msg.role === "user";
    const isAssistant = msg.role === "assistant";
    let providerModel: { provider_id?: string; model_name?: string } | null =
      null;
    let meta: DiscussMessageMetadata | null = null;
    if (isAssistant && msg.metadata) {
      try {
        const parsed =
          typeof msg.metadata === "string"
            ? JSON.parse(msg.metadata)
            : msg.metadata;
        meta = parsed as DiscussMessageMetadata;
        if (parsed?.provider_model) providerModel = parsed.provider_model;
      } catch {
        providerModel = null;
      }
    }
    const sources = meta?.sources ?? [];
    return (
      <div
        key={msg.id}
        className={`mb-4 flex ${isUser ? "justify-end" : "justify-start"}`}
      >
        <div className="max-w-[85%]">
          <div
            className={`rounded-2xl px-4 py-3 ${
              isUser
                ? "bg-primary text-on-primary rounded-br-md"
                : "neo-pressed text-on-surface rounded-bl-md"
            }`}
          >
            <p className="text-sm whitespace-pre-wrap break-words">
              {msg.content}
            </p>
            {isAssistant && (providerModel || msg.metadata) && (
              <div className="mt-2 flex items-center gap-2 text-[10px] text-on-surface-variant">
                <Icon name="smart_toy" size={12} />
                <span>
                  {providerModel?.model_name
                    ? providerModel.model_name
                    : providerModel?.provider_id
                      ? providerModel.provider_id
                      : "AI response"}
                </span>
              </div>
            )}
            {/* Repository source citations: the exact files whose contents
                entered the model prompt for this answer. */}
            {isAssistant && sources.length > 0 && (
              <div className="mt-2 pt-2 border-t border-outline-variant/30 dark:border-white/10">
                <div className="flex items-center gap-1.5 text-[10px] font-medium uppercase text-on-surface-variant mb-1">
                  <Icon name="folder_open" size={11} />
                  Sources ({sources.length})
                </div>
                <div className="flex flex-wrap gap-1">
                  {sources.map((source) => (
                    <span
                      key={source.path}
                      title={`${source.reason} · ${source.excerpt_bytes}B`}
                      className="inline-flex items-center gap-1 text-[10px] neo-raised rounded-full px-2 py-0.5 text-on-surface-variant max-w-full"
                    >
                      <Icon name="description" size={10} className="shrink-0 text-primary" />
                      <span className="truncate">{source.path}</span>
                    </span>
                  ))}
                </div>
              </div>
            )}
            {isAssistant && meta?.sources_degraded && (
              <div className="mt-2 text-[10px] text-amber-600 dark:text-amber-400 flex items-center gap-1">
                <Icon name="warning" size={11} />
                Grounding degraded: answer used only file names, not source contents.
              </div>
            )}
            {isAssistant && msg.mission_ref && (
              <div className="mt-2 flex items-center gap-2 text-xs text-on-surface-variant">
                <Icon name="terminal" size={12} />
                <span>Mission: {msg.mission_ref}</span>
                <button
                  onClick={() => onOpenMission(msg.mission_ref!)}
                  className="underline hover:opacity-80"
                >
                  View
                </button>
              </div>
            )}
            {msg.mission_ref && !isAssistant && (
              <div className="mt-2 flex items-center gap-2 text-xs text-on-surface-variant">
                <Icon name="terminal" size={12} />
                <span>Mission: {msg.mission_ref}</span>
                <button
                  onClick={() => onOpenMission(msg.mission_ref!)}
                  className="underline hover:opacity-80"
                >
                  View
                </button>
              </div>
            )}
          </div>
          <p className="text-[10px] text-on-surface-variant mt-0.5 px-1 flex items-center gap-2">
            <span>{formatTime(msg.created_at_ms)}</span>
            {isAssistant && (
              <button
                onClick={() => {
                  setDecisionDraft({
                    messageId: msg.id,
                    excerpt: msg.content.slice(0, 200),
                  });
                  setDecisionText("");
                  setDecisionRationale("");
                }}
                className="underline hover:text-primary"
                title="Accept a decision derived from this answer"
              >
                Accept as Decision
              </button>
            )}
          </p>
        </div>
      </div>
    );
  }

  // ── Main render ────────────────────────────────────────────────────────────

  if (status === "no_project") {
    return (
      <main className="flex-1 flex items-center justify-center">
        <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
          <Icon name="forum" size={40} className="text-primary mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-on-surface mb-2">
            Open a Project
          </h3>
          <p className="text-sm text-on-surface-variant">
            Select a project to discuss its architecture, design, and code.
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
          <h3 className="text-lg font-semibold text-on-surface mb-2">
            Daemon Unavailable
          </h3>
          <p className="text-sm text-on-surface-variant">
            The AgentCode daemon is not responding. Discussions are persisted and
            will be available when the daemon restarts.
          </p>
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 flex overflow-hidden">
      {/* Conversation List */}
      <aside className="w-64 shrink-0 flex flex-col border-r border-outline-variant/40 dark:border-white/5 bg-surface/50">
        <div className="p-3 border-b border-outline-variant/40 dark:border-white/5">
          <button
            onClick={handleNewChat}
            className="w-full neo-button rounded-xl py-2.5 flex items-center justify-center gap-2 text-sm font-medium text-primary"
          >
            <Icon name="add" size={18} />
            New Discussion
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
                  {formatTime(conv.updated_at_ms)}
                </p>
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
              No discussions yet. Start a new discussion.
            </div>
          )}
        </div>
      </aside>

      {/* Chat Area */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {!activeConvId ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
              <Icon name="forum" size={40} className="text-primary mx-auto mb-4" />
              <h3 className="text-lg font-semibold text-on-surface mb-2">
                Select a Discussion
              </h3>
              <p className="text-sm text-on-surface-variant">
                Choose a discussion from the sidebar or start a new one.
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
                  <span className="text-xs font-bold uppercase text-violet-600 dark:text-violet-400">
                    Discuss Mode
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
              {convDetail?.messages?.map(renderMessage)}
              {thinking && (
                <div className="flex justify-start mb-4">
                  <div className="neo-pressed rounded-2xl px-4 py-3 rounded-bl-md">
                    <div className="flex items-center gap-2 text-sm text-on-surface-variant">
                      <Icon name="autorenew" size={16} className="animate-spin" />
                      <span>Thinking...</span>
                    </div>
                  </div>
                </div>
              )}
              {(!convDetail || convDetail.messages.length === 0) && !thinking && (
                <div className="text-center text-xs text-on-surface-variant py-10">
                  Ask a question about your project, its architecture, or design.
                </div>
              )}
              <div ref={messagesEndRef} />
            </div>

            {/* Pending Attachments */}
            {pendingAttachments.length > 0 && (
              <div className="shrink-0 px-6 py-2 border-t border-outline-variant/40 dark:border-white/5">
                <div className="flex flex-wrap gap-2">
                  {pendingAttachments.map((a) => {
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
                  })}
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
                    placeholder="Ask about the project..."
                    value={input}
                    disabled={sending || thinking}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                    rows={1}
                  />
                  <button
                    onClick={handleAttach}
                    disabled={sending || thinking}
                    className="w-9 h-9 rounded-full neo-button flex items-center justify-center text-on-surface-variant hover:text-primary"
                    title="Attach file"
                    aria-label="Attach file"
                  >
                    <Icon name="attach_file" size={18} />
                  </button>
                  <button
                    onClick={handleSend}
                    disabled={sending || thinking || !input.trim()}
                    className="w-9 h-9 rounded-full bg-primary flex items-center justify-center text-on-primary disabled:opacity-50 active:scale-95 transition-all"
                    aria-label="Send message"
                  >
                    <Icon
                      name={sending || thinking ? "autorenew" : "send"}
                      size={18}
                      className={sending || thinking ? "animate-spin" : ""}
                    />
                  </button>
                </div>
              </div>
              {/* Structured plan panel (Turn Into Plan result) */}
              {plan && (
                <div className="mt-3 neo-pressed rounded-2xl p-4">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <Icon name="checklist" size={16} className="text-primary" />
                      <span className="text-sm font-semibold text-on-surface">
                        Plan
                      </span>
                    </div>
                    {plan.promoted_mission_id && (
                      <button
                        onClick={() => onOpenMission(plan.promoted_mission_id!)}
                        className="text-xs underline text-primary"
                      >
                        Mission {plan.promoted_mission_id.slice(0, 16)}…
                      </button>
                    )}
                  </div>
                  <p className="text-sm text-on-surface mb-2">
                    <span className="font-medium">Goal:</span> {plan.goal}
                  </p>
                  {plan.requirements.length > 0 && (
                    <div className="mb-2">
                      <p className="text-[10px] uppercase font-medium text-on-surface-variant mb-1">
                        Requirements
                      </p>
                      <ul className="text-xs text-on-surface space-y-0.5">
                        {plan.requirements.map((r, i) => (
                          <li key={i}>• {r}</li>
                        ))}
                      </ul>
                    </div>
                  )}
                  {plan.open_questions.length > 0 && (
                    <div className="mb-2">
                      <p className="text-[10px] uppercase font-medium text-on-surface-variant mb-1">
                        Open Questions
                      </p>
                      <ul className="text-xs text-on-surface space-y-0.5">
                        {plan.open_questions.map((q, i) => (
                          <li key={i}>? {q}</li>
                        ))}
                      </ul>
                    </div>
                  )}
                  {(plan.constraints?.length ?? 0) > 0 && (
                    <div className="mb-2">
                      <p className="text-[10px] uppercase font-medium text-on-surface-variant mb-1">
                        Constraints
                      </p>
                      <ul className="text-xs text-on-surface space-y-0.5">
                        {plan.constraints.map((c, i) => (
                          <li key={i}>! {c}</li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              )}

              {/* Accepted decisions panel */}
              {decisions.length > 0 && (
                <div className="mt-3 neo-pressed rounded-2xl p-4">
                  <div className="flex items-center gap-2 mb-2">
                    <Icon name="gavel" size={16} className="text-primary" />
                    <span className="text-sm font-semibold text-on-surface">
                      Accepted Decisions ({decisions.length})
                    </span>
                  </div>
                  <ul className="space-y-1.5">
                    {decisions.map((d) => (
                      <li key={d.id} className="text-xs">
                        <span className="text-on-surface font-medium">
                          {d.decision}
                        </span>
                        <span className="text-on-surface-variant block pl-2">
                          {d.rationale}
                        </span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              {/* Decision acceptance dialog (explicit user action only) */}
              {decisionDraft && (
                <div className="mt-3 neo-raised rounded-2xl p-4 border border-primary/30">
                  <div className="flex items-center gap-2 mb-2">
                    <Icon name="gavel" size={16} className="text-primary" />
                    <span className="text-sm font-semibold text-on-surface">
                      Accept as Decision
                    </span>
                  </div>
                  <p className="text-[10px] text-on-surface-variant mb-2 truncate">
                    From: “{decisionDraft.excerpt}…”
                  </p>
                  <input
                    className="neo-input rounded-xl px-3 py-2 w-full text-sm mb-2"
                    placeholder="Decision (e.g. Use SQLite WAL mode)"
                    value={decisionText}
                    onChange={(e) => setDecisionText(e.target.value)}
                  />
                  <input
                    className="neo-input rounded-xl px-3 py-2 w-full text-sm mb-3"
                    placeholder="Rationale (optional)"
                    value={decisionRationale}
                    onChange={(e) => setDecisionRationale(e.target.value)}
                  />
                  <div className="flex gap-2 justify-end">
                    <button
                      onClick={() => setDecisionDraft(null)}
                      className="neo-button rounded-xl px-3 py-1.5 text-xs text-on-surface-variant"
                    >
                      Cancel
                    </button>
                    <button
                      onClick={handleAcceptDecision}
                      disabled={planBusy || !decisionText.trim()}
                      className="rounded-xl px-4 py-1.5 text-xs font-medium bg-primary text-on-primary disabled:opacity-50"
                    >
                      {planBusy ? "Accepting…" : "Accept Decision"}
                    </button>
                  </div>
                </div>
              )}

              {/* Discussion actions: Turn Into Plan → Execute Plan (Doc 06 §23-24) */}
              <div className="flex justify-end mt-2 gap-2">
                <button
                  onClick={handleTurnIntoPlan}
                  disabled={sending || thinking || planBusy || !convDetail?.messages?.some((m) => m.role === "assistant")}
                  className="neo-button rounded-xl px-4 py-2 text-sm font-medium text-violet-600 dark:text-violet-400 flex items-center gap-2 disabled:opacity-50"
                  aria-label="Turn this discussion into a structured plan"
                >
                  <Icon name={planBusy ? "autorenew" : "checklist"} size={16} className={planBusy ? "animate-spin" : ""} />
                  Turn Into Plan
                </button>
                <button
                  onClick={handleExecutePlan}
                  disabled={sending || thinking || planBusy || !plan}
                  className="neo-button rounded-xl px-4 py-2 text-sm font-medium text-violet-600 dark:text-violet-400 flex items-center gap-2 disabled:opacity-50"
                  aria-label="Execute the structured plan as a mission with full context"
                >
                  <Icon name="terminal" size={16} />
                  Execute Plan
                </button>
              </div>
            </div>
          </>
        )}
      </div>
    </main>
  );
}