import { useEffect, useState, useRef, useCallback } from "react";
import type { KeyboardEvent } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { Project } from "./ProjectContext";
import type { Conversation, ConversationDetail, Message, Attachment, ConversationMode } from "./types";

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
  const [activeConvId, setActiveConvId] = useState<string | null>(null);
  const [convDetail, setConvDetail] = useState<ConversationDetail | null>(null);
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
      return;
    }
    setStatus("loading");
    let cancelled = false;
    (async () => {
      const convs = await daemon.listConversations(project.path);
      if (cancelled) return;
      setConversations(convs);
      // If no active conversation, select the first one
      if (!activeConvId && convs.length > 0) {
        setActiveConvId(convs[0].id);
      }
      setStatus(convs.length > 0 ? "loaded" : "no_chat");
    })();
    return () => { cancelled = true; };
  }, [project]);

  // Load conversation detail when active conversation changes
  useEffect(() => {
    if (!activeConvId) {
      setConvDetail(null);
      setAttachments([]);
      return;
    }
    let cancelled = false;
    (async () => {
      const detail = await daemon.getConversation(activeConvId);
      if (cancelled || !detail) return;
      setConvDetail(detail);
      setAttachments(detail.attachments ?? []);
    })();
    return () => { cancelled = true; };
  }, [activeConvId]);

  // Scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [convDetail?.messages?.length]);

  const refreshConversations = useCallback(async () => {
    if (!project) return;
    const convs = await daemon.listConversations(project.path);
    setConversations(convs);
  }, [project]);

  const refreshActive = useCallback(async () => {
    if (!activeConvId) return;
    const detail = await daemon.getConversation(activeConvId);
    if (detail) {
      setConvDetail(detail);
      setAttachments(detail.attachments ?? []);
    }
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
    // Use the latest user message as the goal
    const lastUserMsg = convDetail.messages
      .filter((m) => m.role === "user" && !m.mission_ref)
      .pop();
    const goal = lastUserMsg?.content || input.trim();
    if (!goal) return;
    setBusy(true);
    setError(null);
    const result = await daemon.submitGoalFromConversation(
      activeConvId,
      goal,
      pendingAttachments.map((a) => a.id)
    );
    if (result.ok) {
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
    return (
      <div key={msg.id} className={`flex ${isUser ? "justify-end" : "justify-start"} mb-4`}>
        <div className={`max-w-[80%] ${isUser ? "order-1" : "order-1"}`}>
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

  function renderMissionCard() {
    if (!convDetail?.current_mission_id) return null;
    return (
      <div className="neo-raised rounded-2xl p-4 mb-4 border border-primary/20">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Icon name="terminal" size={18} className="text-primary" />
            <span className="text-sm font-semibold text-on-surface">Active Mission</span>
          </div>
          <button
            onClick={() => onOpenMission(convDetail.current_mission_id!)}
            className="neo-button px-3 py-1.5 rounded-lg text-xs text-primary font-medium"
          >
            View Details
          </button>
        </div>
        <p className="text-xs text-on-surface-variant mt-1">
          Mission {convDetail.current_mission_id}
        </p>
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
      <aside className="w-64 shrink-0 flex flex-col border-r border-outline-variant/40 dark:border-white/5 bg-surface/50">
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
              {daemonConnected && (
                <span className="flex items-center gap-1.5 text-[10px] text-emerald-600 dark:text-emerald-400">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                  Connected
                </span>
              )}
            </div>

            {/* Messages */}
            <div className="flex-1 overflow-y-auto px-6 py-4">
              {renderMissionCard()}
              {convDetail?.messages.map(renderMessage)}
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
                  >
                    <Icon name="attach_file" size={18} />
                  </button>
                  <button
                    onClick={handleSend}
                    disabled={submitting || !input.trim()}
                    className="w-9 h-9 rounded-full bg-primary flex items-center justify-center text-on-primary disabled:opacity-50 active:scale-95 transition-all"
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
                    disabled={busy || (!convDetail?.messages?.some((m) => m.role === "user" && !m.mission_ref) && !input.trim())}
                    className="neo-button rounded-xl px-4 py-2 text-sm font-medium text-primary flex items-center gap-2 disabled:opacity-50"
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