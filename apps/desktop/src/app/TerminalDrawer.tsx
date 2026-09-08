import { useCallback, useEffect, useRef, useState } from "react";
import { daemon } from "./daemon";
import { Icon } from "./Icon";

// Global terminal drawer — Codex bottom-pane parity with a REAL terminal
// model: the top bar holds TERMINAL TABS (user-created, closable), not
// one page per command.  Each tab is one continuous shell transcript:
// every command echoes inline and its output appends in order, exactly
// like a terminal window.  Clicking a tab shows its stored output —
// nothing ever reruns.  Agent commands surface as read-only robot tabs.
const MIN_H = 200;
const MAX_H = 780;
const DEFAULT_H = 340;

interface TerminalSession {
  session_id: string;
  argv: string[];
  cwd: string;
  source?: string;
  alive: boolean;
  exit_code: number | null;
  captured_lines: number;
  mission_id: string | null;
}

/** A user terminal tab: continuous output + its own command history. */
interface TerminalTab {
  id: string;
  title: string;
  lines: string[];
  sessions: string[];
  cursor: number;
  history: string[];
  historyIdx: number;
  agent?: boolean;
}

let tabSeq = 1;
const newTab = (userTabs: number): TerminalTab => ({
  id: `t${tabSeq++}`,
  title: `sh ${userTabs + 1}`,
  lines: [],
  sessions: [],
  cursor: 0,
  history: [],
  historyIdx: -1,
});

export function TerminalDrawer({
  open,
  onOpenChange,
  projectPath,
}: {
  open: boolean;
  onOpenChange(open: boolean): void;
  projectPath: string | null;
}) {
  const [height, setHeight] = useState(DEFAULT_H);
  const [cmd, setCmd] = useState("");
  const [tabs, setTabs] = useState<TerminalTab[]>([newTab(0)]);
  const [activeId, setActiveId] = useState<string>("t1");
  const [error, setError] = useState<string | null>(null);
  const pollRef = useRef<number | null>(null);
  const outputRef = useRef<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const resizeState = useRef<{ startY: number; startH: number } | null>(null);
  // Agent sessions seen in the daemon list become read-only tabs.
  const agentTabs = useRef<Map<string, TerminalTab>>(new Map());

  const active = tabs.find((t) => t.id === activeId) ?? tabs[0];

  // Poll the ACTIVE tab's latest session tail + the daemon session list
  // (for agent tabs) while the drawer is open.
  useEffect(() => {
    const stop = () => {
      if (pollRef.current !== null) {
        window.clearInterval(pollRef.current);
        pollRef.current = null;
      }
    };
    if (!open) {
      stop();
      return;
    }
    pollRef.current = window.setInterval(
      () =>
        void (async () => {
          const list = await daemon.terminalList();
          const sessions: TerminalSession[] = list.ok && list.list ? list.list.sessions : [];
          // Sync agent sessions (mirrored agent commands) as read-only tabs.
          const known = new Set(tabs.filter((t) => t.agent).map((t) => t.sessions[0]).filter(Boolean));
          const agentIds = sessions.filter((s) => s.source === "agent").map((s) => s.session_id);
          const added = agentIds.filter((id) => !known.has(id) && !agentTabs.current.has(id));
          if (added.length > 0) {
            setTabs((prev) => {
              const next = [...prev];
              for (const sid of added) {
                const t: TerminalTab = {
                  id: `agent-${sid}`,
                  title: sid.slice(0, 10),
                  lines: [],
                  sessions: [sid],
                  cursor: 0,
                  history: [],
                  historyIdx: -1,
                  agent: true,
                };
                agentTabs.current.set(sid, t);
                next.push(t);
              }
              return next;
            });
          }
          // Poll the active tab's latest session for NEW lines only.
          const sid = active?.sessions[active.sessions.length - 1];
          if (sid) {
            const session = sessions.find((s) => s.session_id === sid);
            const tail = await daemon.terminalTail(sid, active.cursor);
            if (tail.ok && tail.tail && tail.tail.lines.length > 0) {
              const lines = tail.tail.lines;
              const cursor = tail.tail.cursor;
              setTabs((prev) =>
                prev.map((t) =>
                  t.id === active.id
                    ? { ...t, lines: [...t.lines, ...lines], cursor }
                    : t
                )
              );
            } else if (tail.ok && tail.tail) {
              // keep cursor fresh even with no new lines
              setTabs((prev) =>
                prev.map((t) => (t.id === active.id ? { ...t, cursor: tail.tail!.cursor } : t))
              );
            }
            void session;
          }
        })(),
      700
    );
    return stop;
  }, [open, activeId, tabs, active?.id, active?.cursor, active?.sessions]);

  // Auto-scroll to the newest line like a real terminal.
  useEffect(() => {
    const el = outputRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [active?.lines.length, activeId, open]);

  // Focus the prompt when the drawer opens.
  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  const run = useCallback(() => {
    void (async () => {
      setError(null);
      const command = cmd.trim();
      if (!command || !active) return;
      const cwd = projectPath ?? (await daemon.homeDir()) ?? "/";
      const res = await daemon.terminalStart(null, [command], cwd, "shell");
      if (!res.ok || !res.session) {
        setError(res.error ?? "failed to start");
        return;
      }
      const sid = res.session.session_id;
      // ONE continuous transcript: echo the command inline, then stream
      // its output after it — like a terminal window.
      setTabs((prev) =>
        prev.map((t) =>
          t.id === active.id
            ? {
                ...t,
                lines: [...t.lines, `$ ${command}`],
                sessions: [...t.sessions, sid],
                cursor: 0,
                history: [command, ...t.history].slice(0, 50),
                historyIdx: -1,
              }
            : t
        )
      );
      setCmd("");
    })();
  }, [cmd, projectPath, active]);

  const cancelActive = () =>
    void (async () => {
      if (!active) return;
      const sid = active.sessions[active.sessions.length - 1];
      if (!sid) return;
      const res = await daemon.terminalCancel(sid);
      if (!res.ok) setError(res.error ?? "cancel failed");
    })();

  const closeTab = (id: string) =>
    void (async () => {
      // Cancel any live session owned by the tab, then drop the tab and
      // its output (the terminal session ring lives until the daemon
      // reaps it; the tab no longer displays it).
      const tab = tabs.find((t) => t.id === id);
      if (tab && tab.agent) agentTabs.current.delete(tab.sessions[0]);
      if (tab) {
        for (const sid of tab.sessions) {
          const list = await daemon.terminalList();
          const s = list.ok && list.list ? list.list.sessions.find((x) => x.session_id === sid) : undefined;
          if (s?.alive) await daemon.terminalCancel(sid);
        }
      }
      setTabs((prev) => {
        const next = prev.filter((t) => t.id !== id);
        if (next.length === 0) return [newTab(0)];
        return next;
      });
      setActiveId((cur) => {
        const remaining = tabs.filter((t) => t.id !== id);
        if (remaining.length === 0) return "t" + tabSeq;
        return cur === id ? remaining[remaining.length - 1].id : cur;
      });
    })();

  const addTab = () => {
    setTabs((prev) => {
      const userTabs = prev.filter((t) => !t.agent).length;
      const t = newTab(userTabs);
      setActiveId(t.id);
      return [...prev, t];
    });
  };

  // ── Drag-to-resize (top edge handle) ─────────────────────────────
  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      const st = resizeState.current;
      if (!st) return;
      const next = Math.min(MAX_H, Math.max(MIN_H, st.startH + (st.startY - e.clientY)));
      setHeight(next);
    };
    const onUp = () => {
      resizeState.current = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, []);

  const startResize = (e: React.MouseEvent) => {
    resizeState.current = { startY: e.clientY, startH: height };
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";
  };

  // THE drawer only exists while open: closed = nothing rendered.
  if (!open) return null;

  return (
    <div
      className="shrink-0 flex flex-col border-t border-white/10 bg-[#0c0f14] text-[#d6e2f0] font-mono text-[13px]"
      style={{ height }}
    >
      {/* Drag handle */}
      <div
        onMouseDown={startResize}
        className="shrink-0 h-1.5 cursor-row-resize hover:bg-primary/60 transition-colors"
        title="Drag to resize"
      />

      {/* Slim chrome row: cwd · TERMINAL TABS (user-created) · close */}
      <div className="shrink-0 flex items-center gap-2 px-3 h-9 bg-[#11151c] border-b border-white/5">
        <Icon name="terminal" size={13} className="text-primary shrink-0" />
        <span className="text-[11px] text-white/50 truncate max-w-[22%]" title={projectPath ?? "home"}>
          {projectPath ? projectPath.split("/").slice(-2).join("/") : "~"}
        </span>
        {/* Terminal tabs — one per window the user opened, closable;
            commands live INSIDE the tab as a continuous transcript. */}
        <div className="flex-1 flex items-center gap-1 overflow-x-auto no-scrollbar">
          {tabs.map((t) => (
            <span
              key={t.id}
              className={`shrink-0 text-[11px] px-2 py-0.5 rounded-md flex items-center gap-1 group ${
                activeId === t.id
                  ? "bg-primary/25 text-white"
                  : "text-white/40 hover:text-white/80 hover:bg-white/5"
              }`}
            >
              {t.agent && <Icon name="smart_toy" size={10} className="text-primary" />}
              <button
                onClick={() => setActiveId(t.id)}
                className="outline-none"
                title={t.agent ? `agent session — stored output` : `${t.title} — stored output (clicking never reruns)`}
              >
                {t.title}
              </button>
              <button
                onClick={() => void closeTab(t.id)}
                className="opacity-0 group-hover:opacity-100 text-white/40 hover:text-red-400 transition-opacity"
                title="Close this terminal tab"
              >
                <Icon name="close" size={10} />
              </button>
            </span>
          ))}
          <button
            onClick={addTab}
            className="shrink-0 px-1.5 py-0.5 rounded-md text-white/40 hover:text-white hover:bg-white/5"
            title="Open a new terminal tab"
          >
            +
          </button>
        </div>
        <button
          onClick={() => onOpenChange(false)}
          className="p-1 rounded text-white/40 hover:text-white hover:bg-white/10"
          title="Hide terminal"
        >
          <Icon name="close" size={14} />
        </button>
      </div>

      {/* Full-bleed continuous transcript for the ACTIVE tab */}
      <div
        ref={outputRef}
        className="flex-1 overflow-y-auto px-3 py-2 leading-[1.45] whitespace-pre-wrap break-all selection:bg-primary/40"
      >
        {active && active.lines.length > 0 ? (
          active.lines.map((l, i) => (
            <div key={i} className={l.startsWith("$ ") ? "text-primary/90" : ""}>
              {l}
            </div>
          ))
        ) : (
          <p className="text-white/30 italic">
            {active?.agent
              ? "agent session — waiting for output…"
              : "Type a command below — it runs in your login shell, and every command + its output stays in this tab's transcript."}
          </p>
        )}
      </div>

      {/* Error line */}
      {error && (
        <p className="shrink-0 px-3 pb-1 text-[11px] text-red-400 flex items-center gap-1.5">
          <Icon name="error" size={12} fill /> {error}
        </p>
      )}

      {/* Prompt at the bottom — like a real shell */}
      <div className="shrink-0 flex items-center gap-2 px-3 h-10 bg-[#11151c] border-t border-white/5">
        <span className="text-emerald-400 shrink-0">$</span>
        <input
          ref={inputRef}
          value={cmd}
          disabled={active?.agent}
          onChange={(e) => setCmd(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") run();
            const h = active?.history ?? [];
            if (e.key === "ArrowUp" && h.length > 0) {
              e.preventDefault();
              const next = Math.min((active?.historyIdx ?? -1) + 1, h.length - 1);
              setTabs((prev) => prev.map((t) => (t.id === active.id ? { ...t, historyIdx: next } : t)));
              setCmd(h[next]);
            }
            if (e.key === "ArrowDown") {
              e.preventDefault();
              const next = (active?.historyIdx ?? -1) - 1;
              setTabs((prev) => prev.map((t) => (t.id === active.id ? { ...t, historyIdx: next } : t)));
              setCmd(next >= 0 ? h[next] : "");
            }
          }}
          placeholder={active?.agent ? "read-only agent session" : "type any command — pipes, &&, env vars all work…"}
          className="flex-1 bg-transparent text-[13px] text-[#d6e2f0] outline-none placeholder:text-white/25 disabled:opacity-50"
          spellCheck={false}
        />
        <div className="flex items-center gap-1 shrink-0">
          <button
            onClick={cancelActive}
            className="text-[11px] px-2 py-1 rounded bg-red-500/15 text-red-400 hover:bg-red-500/25"
            title="Stop the running process"
          >
            stop
          </button>
          <button
            onClick={run}
            className="text-[11px] px-2 py-1 rounded bg-primary/20 text-primary hover:bg-primary/30"
            title="Run (Enter)"
          >
            run
          </button>
        </div>
      </div>
    </div>
  );
}
