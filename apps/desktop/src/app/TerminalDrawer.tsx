import { useCallback, useEffect, useRef, useState } from "react";
import { daemon } from "./daemon";
import { Icon } from "./Icon";

// Global terminal drawer — Codex bottom-pane parity with a REAL terminal
// feel: a full-bleed dark output surface, the prompt at the bottom like a
// shell, one slim chrome row (cwd + session tabs + live indicator), and a
// drag-to-resize handle on the top edge.  Commands run through the user's
// login shell in the open project's cwd (Codex /shell) — no mission needed.
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
  const [sessions, setSessions] = useState<TerminalSession[]>([]);
  const [activeSession, setActiveSession] = useState<string | null>(null);
  const [lines, setLines] = useState<string[]>([]);
  const [cursor, setCursor] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<string[]>([]);
  const [historyIdx, setHistoryIdx] = useState(-1);
  const pollRef = useRef<number | null>(null);
  const outputRef = useRef<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const resizeState = useRef<{ startY: number; startH: number } | null>(null);

  // Poll sessions + active tail while the drawer is open.
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
          if (list.ok && list.list) setSessions(list.list.sessions);
          if (activeSession) {
            const tail = await daemon.terminalTail(activeSession, cursor);
            if (tail.ok && tail.tail) {
              if (tail.tail.lines.length > 0)
                setLines((prev) => [...prev, ...tail.tail!.lines]);
              setCursor(tail.tail.cursor);
            }
          }
        })(),
      700
    );
    return stop;
  }, [open, activeSession, cursor]);

  // Auto-scroll to the newest line like a real terminal.
  useEffect(() => {
    const el = outputRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines]);

  const run = useCallback(() => {
    void (async () => {
      setError(null);
      const command = cmd.trim();
      if (!command) return;
      const cwd = projectPath ?? (await daemon.homeDir()) ?? "/";
      const res = await daemon.terminalStart(null, [command], cwd, "shell");
      if (!res.ok || !res.session) {
        setError(res.error ?? "failed to start");
        return;
      }
      setHistory((prev) => [command, ...prev].slice(0, 50));
      setHistoryIdx(-1);
      setActiveSession(res.session.session_id);
      setLines([`$ ${command}`]);
      setCursor(0);
      setCmd("");
      const list = await daemon.terminalList();
      if (list.ok && list.list) setSessions(list.list.sessions);
    })();
  }, [cmd, projectPath]);

  const cancel = () =>
    void (async () => {
      if (!activeSession) return;
      const res = await daemon.terminalCancel(activeSession);
      if (!res.ok) setError(res.error ?? "cancel failed");
    })();

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

  // Focus the prompt when the drawer opens.
  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  const active = activeSession ? sessions.find((s) => s.session_id === activeSession) : undefined;

  // THE drawer only exists while open: closed = nothing rendered (the
  // toggle and the cross both collapse it instantly; no hidden element
  // can steal focus or sit in the layout).
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

      {/* Slim chrome row: cwd · session tabs · live · close */}
      <div className="shrink-0 flex items-center gap-2 px-3 h-9 bg-[#11151c] border-b border-white/5">
        <Icon name="terminal" size={13} className="text-primary shrink-0" />
        <span className="text-[11px] text-white/50 truncate max-w-[26%]" title={projectPath ?? "home"}>
          {projectPath ? projectPath.split("/").slice(-2).join("/") : "~"}
        </span>
        <div className="flex-1 flex items-center gap-1 overflow-x-auto no-scrollbar">
          {sessions.map((s) => (
            <button
              key={s.session_id}
              onClick={() => {
                setActiveSession(s.session_id);
                setLines([
                  `$ ${s.argv.filter(Boolean).join(" ") || s.session_id}  (session ${s.session_id.slice(0, 8)})`,
                ]);
                setCursor(0);
              }}
              className={`shrink-0 text-[11px] px-2 py-0.5 rounded-md flex items-center gap-1 ${
                activeSession === s.session_id
                  ? "bg-primary/25 text-white"
                  : "text-white/40 hover:text-white/80 hover:bg-white/5"
              }`}
              title={s.argv.join(" ")}
            >
              {s.source === "agent" && <Icon name="smart_toy" size={10} className="text-primary" />}
              {s.alive && <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse shrink-0" />}
              {(s.argv.filter(Boolean).slice(-1)[0] ?? s.session_id).slice(0, 28)}
            </button>
          ))}
        </div>
        {active?.exit_code != null && (
          <span
            className={`text-[10px] px-1.5 py-0.5 rounded ${
              active.exit_code === 0 ? "bg-emerald-500/15 text-emerald-400" : "bg-red-500/15 text-red-400"
            }`}
          >
            exit {active.exit_code}
          </span>
        )}
        <button
          onClick={() => onOpenChange(false)}
          className="p-1 rounded text-white/40 hover:text-white hover:bg-white/10"
          title="Hide terminal"
        >
          <Icon name="close" size={14} />
        </button>
      </div>

      {/* Full-bleed output */}
      <div
        ref={outputRef}
        className="flex-1 overflow-y-auto px-3 py-2 leading-[1.45] whitespace-pre-wrap break-all selection:bg-primary/40"
      >
        {activeSession ? (
          lines.length === 0 ? (
            <p className="text-white/30 italic">waiting for output…</p>
          ) : (
            lines.map((l, i) => (
              <div key={i} className={l.startsWith("$ ") ? "text-primary/90" : ""}>
                {l}
              </div>
            ))
          )
        ) : (
          <p className="text-white/30 italic">
            Type a command below — anything your shell can run. Agent commands appear as tabs.
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
          onChange={(e) => setCmd(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") run();
            if (e.key === "ArrowUp" && history.length > 0) {
              e.preventDefault();
              const next = Math.min(historyIdx + 1, history.length - 1);
              setHistoryIdx(next);
              setCmd(history[next]);
            }
            if (e.key === "ArrowDown") {
              e.preventDefault();
              const next = historyIdx - 1;
              setHistoryIdx(next);
              setCmd(next >= 0 ? history[next] : "");
            }
          }}
          placeholder="type any command — pipes, &&, env vars all work…"
          className="flex-1 bg-transparent text-[13px] text-[#d6e2f0] outline-none placeholder:text-white/25"
          spellCheck={false}
        />
        <div className="flex items-center gap-1 shrink-0">
          {active?.alive && (
            <button
              onClick={cancel}
              className="text-[11px] px-2 py-1 rounded bg-red-500/15 text-red-400 hover:bg-red-500/25"
              title="Stop the running process"
            >
              stop
            </button>
          )}
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
