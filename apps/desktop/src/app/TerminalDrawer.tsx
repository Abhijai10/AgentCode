import { useCallback, useEffect, useRef, useState } from "react";
import { daemon } from "./daemon";
import { Icon } from "./Icon";

// Global terminal drawer — REAL terminal model: the top bar holds
// TERMINAL TABS (user-created, closable), not one page per command.
// Each tab is one continuous shell transcript: every command echoes
// inline and its output appends in order, like a terminal window.
// Clicking a tab shows its STORED output — nothing ever reruns.  Tabs
// and their output are restored after an app reload (sessions live in
// the daemon until the tab is closed).  Agent commands surface as
// read-only robot tabs.
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

/** A user terminal tab: one continuous transcript + its own history. */
interface TerminalTab {
  id: string;
  title: string;
  lines: string[];
  sessions: string[];
  cursors: Record<string, number>;
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
  cursors: {},
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
  const agentTabs = useRef<Set<string>>(new Set());
  const restoredRef = useRef(false);
  // Mirrors so the 700ms poll never restarts on every keystroke/line.
  const tabsRef = useRef(tabs);
  tabsRef.current = tabs;
  const activeIdRef = useRef(activeId);
  activeIdRef.current = activeId;

  const active = tabs.find((t) => t.id === activeId) ?? tabs[0];

  // Poll while the drawer is open: restore daemon-side sessions once
  // (after an app reload), sync new agent sessions, and stream new
  // output for EVERY session of the active tab.
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
    const tick = async () => {
      const list = await daemon.terminalList();
      const sessions: TerminalSession[] =
        list.ok && list.list ? list.list.sessions : [];
      if (!restoredRef.current) {
        restoredRef.current = true;
        const cur = tabsRef.current;
        const owned = new Set(cur.flatMap((t) => t.sessions));
        const userTabs: TerminalTab[] = [];
        const agentAdded: TerminalTab[] = [];
        for (const s of sessions) {
          if (owned.has(s.session_id)) continue;
          if (s.source === "agent") {
            agentTabs.current.add(s.session_id);
            agentAdded.push({
              id: `agent-${s.session_id}`,
              title: s.session_id.slice(0, 8),
              lines: [],
              sessions: [s.session_id],
              cursors: { [s.session_id]: 0 },
              history: [],
              historyIdx: -1,
              agent: true,
            });
          } else if (s.source === "user_shell" && !s.mission_id) {
            userTabs.push({
              id: `s-${s.session_id}`,
              title: `sh ${userTabs.length + 1}`,
              lines: [],
              sessions: [s.session_id],
              cursors: { [s.session_id]: 0 },
              history: [],
              historyIdx: -1,
            });
          }
        }
        if (userTabs.length > 0 || agentAdded.length > 0) {
          setTabs((prev) => {
            // Drop a never-used placeholder when real tabs exist.
            const base = prev.filter(
              (t) => t.sessions.length > 0 || t.lines.length > 0 || t.agent
            );
            return [...base, ...userTabs, ...agentAdded];
          });
          const lastUser = userTabs[userTabs.length - 1];
          if (lastUser) setActiveId(lastUser.id);
        }
      } else {
        const cur = tabsRef.current;
        const owned = new Set(cur.flatMap((t) => t.sessions));
        const added = sessions.filter(
          (s) =>
            s.source === "agent" &&
            !owned.has(s.session_id) &&
            !agentTabs.current.has(s.session_id)
        );
        if (added.length > 0) {
          setTabs((prev) => {
            const ownedNow = new Set(prev.flatMap((t) => t.sessions));
            const next = [...prev];
            for (const s of added) {
              if (ownedNow.has(s.session_id)) continue;
              agentTabs.current.add(s.session_id);
              next.push({
                id: `agent-${s.session_id}`,
                title: s.session_id.slice(0, 8),
                lines: [],
                sessions: [s.session_id],
                cursors: { [s.session_id]: 0 },
                history: [],
                historyIdx: -1,
                agent: true,
              });
            }
            return next;
          });
        }
      }
      // Stream new output for every session of the ACTIVE tab (each
      // session keeps its own cursor, so interleaved commands stream
      // without duplicating or dropping lines).
      const curTabs = tabsRef.current;
      const act = curTabs.find((t) => t.id === activeIdRef.current) ?? curTabs[0];
      if (!act || act.sessions.length === 0) return;
      let changed = false;
      const appended: Record<string, string[]> = {};
      const cursors: Record<string, number> = {};
      for (const sid of act.sessions) {
        const tail = await daemon.terminalTail(sid, act.cursors[sid] ?? 0);
        if (tail.ok && tail.tail) {
          const from = act.cursors[sid] ?? 0;
          if (tail.tail.cursor !== from) {
            changed = true;
            cursors[sid] = tail.tail.cursor;
            if (tail.tail.lines.length > 0) appended[sid] = tail.tail.lines;
          }
        }
      }
      if (changed) {
        setTabs((prev) =>
          prev.map((t) => {
            if (t.id !== act.id) return t;
            let lines = t.lines;
            for (const sid of t.sessions) {
              const l = appended[sid];
              if (l && l.length > 0) lines = [...lines, ...l];
            }
            return { ...t, lines, cursors: { ...t.cursors, ...cursors } };
          })
        );
      }
    };
    pollRef.current = window.setInterval(() => void tick(), 700);
    return stop;
  }, [open]);

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
      const act =
        tabsRef.current.find((t) => t.id === activeIdRef.current) ??
        tabsRef.current[0];
      if (!command || !act || act.agent) return;
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
          t.id === act.id
            ? {
                ...t,
                lines: [...t.lines, `$ ${command}`],
                sessions: [...t.sessions, sid],
                cursors: { ...t.cursors, [sid]: 0 },
                history: [command, ...t.history].slice(0, 50),
                historyIdx: -1,
              }
            : t
        )
      );
      setCmd("");
    })();
  }, [cmd, projectPath]);

  const cancelActive = () =>
    void (async () => {
      const act =
        tabsRef.current.find((t) => t.id === activeIdRef.current) ??
        tabsRef.current[0];
      if (!act) return;
      const sid = act.sessions[act.sessions.length - 1];
      if (!sid) return;
      const res = await daemon.terminalCancel(sid);
      if (!res.ok) setError(res.error ?? "cancel failed");
    })();

  const closeTab = (id: string) =>
    void (async () => {
      // Cancel any live session owned by the tab, then drop the tab and
      // its transcript (the tab no longer displays the stored output).
      const tab = tabsRef.current.find((t) => t.id === id);
      if (tab) {
        if (tab.agent) agentTabs.current.delete(tab.sessions[0]);
        for (const sid of tab.sessions) {
          const list = await daemon.terminalList();
          const s =
            list.ok && list.list
              ? list.list.sessions.find((x) => x.session_id === sid)
              : undefined;
          if (s?.alive) await daemon.terminalCancel(sid);
        }
      }
      const remaining = tabsRef.current.filter((t) => t.id !== id);
      if (remaining.length === 0) {
        const fresh = newTab(0);
        setTabs([fresh]);
        setActiveId(fresh.id);
        return;
      }
      setTabs(remaining);
      if (activeIdRef.current === id)
        setActiveId(remaining[remaining.length - 1].id);
    })();

  const addTab = () => {
    const prev = tabsRef.current;
    const userTabs = prev.filter((t) => !t.agent).length;
    const t = newTab(userTabs);
    setTabs([...prev, t]);
    setActiveId(t.id);
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
          className="flex-1 bg-transparent text-[13px] text-[#d6e2f0] outline-none placeholder:text-white/45 disabled:opacity-50"
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
