import { useEffect, useRef, useState } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { DiscussMessage, DiscussSession } from "./types";

export function DiscussView({ project, branch }: { project: string; branch: string }) {
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [sessions, setSessions] = useState<DiscussSession[]>([]);
  const [activeSession, setActiveSession] = useState<string | null>(null);
  const [messages, setMessages] = useState<DiscussMessage[]>([]);
  const [error, setError] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    const refresh = async () => {
      const s = await daemon.discussListSessions();
      if (cancelled) return;
      setSessions(s);
      if (!activeSession && s.length > 0) {
        setActiveSession(s[0].id);
      }
    };
    refresh();
    const timer = setInterval(refresh, 4000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [activeSession]);

  useEffect(() => {
    if (!activeSession) {
      setMessages([]);
      return;
    }
    let cancelled = false;
    const refresh = async () => {
      const m = await daemon.discussGetMessages(activeSession);
      if (!cancelled) setMessages(m);
    };
    refresh();
    const timer = setInterval(refresh, 4000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [activeSession]);

  const send = async () => {
    const content = input.trim();
    if (!content || busy) return;
    setInput("");
    setBusy(true);
    setError(null);
    if (!activeSession) {
      setError("No discussion session is open yet. Start one from the session list.");
      setBusy(false);
      return;
    }
    const result = await daemon.discussSendMessage(activeSession, content);
    if (result) {
      const m = await daemon.discussGetMessages(activeSession);
      setMessages(m);
    } else {
      setError("The daemon could not record the message. The discuss session is not available on this backend yet.");
      setInput(content);
    }
    setBusy(false);
  };

  return (
    <main className="flex-1 flex flex-col relative px-6 md:px-8 py-6 max-w-5xl mx-auto w-full overflow-hidden">
      <div className="flex-1 neo-raised p-6 flex flex-col mb-6 overflow-hidden min-h-0">
        <div className="flex items-center justify-between mb-6 pb-4 border-b border-outline-variant/40 dark:border-white/5">
          <div>
            <h1 className="text-2xl font-semibold text-on-surface">Repository Discussion</h1>
            <p className="text-sm text-on-surface-variant mt-1">Context: {project} ({branch})</p>
          </div>
          <div className="text-xs text-on-surface-variant">
            {sessions.length} session{sessions.length !== 1 ? "s" : ""} · backend state
          </div>
        </div>

        <div ref={scrollRef} className="flex-1 overflow-y-auto pr-4 flex flex-col gap-4">
          {sessions.length === 0 && (
            <div className="text-center text-on-surface-variant text-sm py-10">
              No discussion sessions yet. The daemon owns discussion state; sessions appear here once the backend records them.
            </div>
          )}
          {sessions.length > 0 && messages.length === 0 && (
            <div className="text-center text-on-surface-variant text-sm py-10">
              This session has no messages yet. Ask a question to start the discussion.
            </div>
          )}
          {messages.map((item) => (
            <div key={item.id} className="neo-pressed rounded-xl p-4">
              <div className="w-full flex items-start gap-3 text-left">
                <span className="mt-0.5 text-primary">
                  <Icon name={item.role === "user" ? "person" : "smart_toy"} size={18} fill />
                </span>
                <div className="flex-1 min-w-0">
                  <div className="flex items-baseline justify-between gap-2">
                    <p className="text-sm font-semibold text-on-surface capitalize">
                      <span className="text-on-surface-variant font-medium mr-2">{item.role}</span>
                    </p>
                    {item.created_at && (
                      <span className="text-xs text-on-surface-variant whitespace-nowrap">{new Date(item.created_at).toLocaleString()}</span>
                    )}
                  </div>
                  <p className="text-sm text-on-surface mt-1 whitespace-pre-wrap">{item.content}</p>
                  {item.context_ref && (
                    <p className="text-[11px] text-on-surface-variant mt-2 font-mono">Context: {item.context_ref}</p>
                  )}
                </div>
              </div>
            </div>
          ))}
          {busy && (
            <div className="flex items-center gap-2 text-sm text-on-surface-variant py-4">
              <Icon name="autorenew" size={16} className="animate-spin" />
              Submitting to daemon...
            </div>
          )}
          {error && (
            <div className="flex items-center gap-2 text-sm text-red-600 dark:text-red-400 py-2">
              <Icon name="error" size={16} fill />
              {error}
            </div>
          )}
        </div>

        <div className="mt-4 relative">
          <div className="relative flex items-center">
            <input
              className="neo-input w-full py-4 pl-14 pr-16 text-sm"
              placeholder="Ask a question or request a change..."
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  send();
                }
              }}
            />
            <button onClick={send} className="absolute right-3 w-10 h-10 rounded-full neo-button flex items-center justify-center text-primary">
              <Icon name="send" size={20} fill />
            </button>
          </div>
          <div className="flex justify-between mt-2 px-2">
            <span className="text-xs text-on-surface-variant">Press Enter to submit</span>
          </div>
        </div>
      </div>
    </main>
  );
}
