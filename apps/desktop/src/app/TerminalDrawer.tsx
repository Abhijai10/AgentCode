import { useEffect, useRef, useState } from "react";
import { daemon } from "./daemon";
import { Icon } from "./Icon";

// Global terminal drawer — Codex bottom-pane parity.  Available from ANY
// view (Codex's /shell + unified exec footer): runs commands through the
// user's login shell in the OPEN PROJECT's cwd (no mission required — the
// exact fix for "Mission has no workspace root"), streams live output, and
// lists every session including agent-mirrored ones (watch-the-agent).
export function TerminalDrawer({
  open,
  onOpenChange,
  projectPath,
}: {
  open: boolean;
  onOpenChange(open: boolean): void;
  projectPath: string | null;
}) {
  const [cmd, setCmd] = useState("");
  const [sessions, setSessions] = useState<
    { session_id: string; argv: string[]; cwd: string; source?: string; alive: boolean; exit_code: number | null; captured_lines: number; mission_id: string | null }[]
  >([]);
  const [activeSession, setActiveSession] = useState<string | null>(null);
  const [lines, setLines] = useState<string[]>([]);
  const [cursor, setCursor] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [evidenceId, setEvidenceId] = useState<string | null>(null);
  const [history, setHistory] = useState<string[]>([]);
  const [historyIdx, setHistoryIdx] = useState(-1);
  const pollRef = useRef<number | null>(null);

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
    pollRef.current = window.setInterval(() => void (async () => {
      const list = await daemon.terminalList();
      if (list.ok && list.list) setSessions(list.list.sessions);
      if (activeSession) {
        const tail = await daemon.terminalTail(activeSession, cursor);
        if (tail.ok && tail.tail) {
          if (tail.tail.lines.length > 0) setLines((prev) => [...prev, ...tail.tail!.lines]);
          setCursor(tail.tail.cursor);
          if (tail.tail.evidence_id) setEvidenceId(tail.tail.evidence_id);
        }
      }
    })(), 1000);
    return stop;
  }, [open, activeSession, cursor]);

  const run = () => void (async () => {
    setError(null);
    const command = cmd.trim();
    if (!command) return;
    // Codex /shell parity: cwd = the OPEN PROJECT (fall back to mission-less
    // home dir), so the terminal works with no mission selected.
    const cwd = projectPath ?? (await daemon.homeDir()) ?? "/";
    const res = await daemon.terminalStart(null, [command], cwd, "shell");
    if (!res.ok || !res.session) {
      setError(res.error ?? "failed to start");
      return;
    }
    setHistory((prev) => [command, ...prev].slice(0, 50));
    setHistoryIdx(-1);
    setActiveSession(res.session.session_id);
    setLines([]);
    setCursor(0);
    setEvidenceId(null);
    setCmd("");
    const list = await daemon.terminalList();
    if (list.ok && list.list) setSessions(list.list.sessions);
  })();

  const cancel = () => void (async () => {
    if (!activeSession) return;
    const res = await daemon.terminalCancel(activeSession);
    if (!res.ok) {
      setError(res.error ?? "cancel failed");
      return;
    }
    if (res.result?.evidence_id) setEvidenceId(res.result.evidence_id);
  })();

  if (!open) return null;

  return (
    <div className="shrink-0 h-72 border-t border-outline-variant/40 dark:border-white/5 bg-surface-container-lowest flex flex-col">
      {/* Drawer header */}
      <div className="shrink-0 flex items-center gap-2 px-4 py-2">
        <Icon name="terminal" size={16} className="text-primary" />
        <span className="text-sm font-semibold text-on-surface">Terminal</span>
        <span className="text-[10px] font-mono text-on-surface-variant truncate max-w-[30%]">
          {projectPath ?? "~/ (no project)"}
        </span>
        {sessions.some((s) => s.alive) && (
          <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" title="live process" />
        )}
        <button
          onClick={() => onOpenChange(false)}
          className="ml-auto p-1.5 rounded-lg text-on-surface-variant hover:bg-surface-variant/40 dark:hover:bg-white/5"
          title="Hide terminal"
        >
          <Icon name="close" size={16} />
        </button>
      </div>

      {/* Command input */}
      <div className="shrink-0 flex gap-2 px-4 pb-2">
        <div className="flex-1 flex items-center gap-2 neo-pressed rounded-xl px-3 py-2">
          <span className="text-primary font-mono text-sm shrink-0">$</span>
          <input
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
            placeholder="any shell command — e.g. ls -la | head, npm test && npm run build"
            className="flex-1 bg-transparent text-sm text-on-surface font-mono outline-none placeholder:text-on-surface-variant/50"
            spellCheck={false}
          />
        </div>
        <button
          onClick={run}
          className="neo-button px-4 py-2 rounded-xl text-sm font-medium text-primary flex items-center gap-1.5"
        >
          <Icon name="play_arrow" size={16} /> Run
        </button>
        {activeSession && sessions.find((s) => s.session_id === activeSession)?.alive && (
          <button
            onClick={cancel}
            className="neo-button px-3 py-2 rounded-xl text-xs font-medium text-red-600 dark:text-red-400 flex items-center gap-1.5"
          >
            <Icon name="stop_circle" size={14} /> Cancel
          </button>
        )}
      </div>

      {error && (
        <p className="shrink-0 px-4 pb-1 text-xs text-red-600 dark:text-red-400 flex items-center gap-1.5">
          <Icon name="error" size={13} fill /> {error}
        </p>
      )}

      {/* Session chips */}
      {sessions.length > 0 && (
        <div className="shrink-0 flex flex-wrap gap-1.5 px-4 pb-2 overflow-x-auto">
          {sessions.map((s) => (
            <button
              key={s.session_id}
              onClick={() => {
                setActiveSession(s.session_id);
                setLines([]);
                setCursor(0);
                setEvidenceId(null);
              }}
              className={`text-[11px] px-2.5 py-1 rounded-full font-mono flex items-center gap-1.5 ${
                activeSession === s.session_id
                  ? "bg-primary/10 text-primary font-semibold"
                  : "neo-pressed text-on-surface-variant"
              }`}
              title={s.argv.join(" ")}
            >
              {s.source === "agent" ? (
                <Icon name="smart_toy" size={11} className="text-primary" />
              ) : (
                <span className={`w-1.5 h-1.5 rounded-full ${s.alive ? "bg-emerald-500" : "bg-outline"}`} />
              )}
              {s.argv.filter(Boolean).slice(-1)[0]?.slice(0, 36) || s.session_id}
              {s.mission_id && <span className="text-[9px] opacity-60">mission</span>}
            </button>
          ))}
        </div>
      )}

      {/* Output */}
      <div className="flex-1 overflow-y-auto px-4 pb-3">
        <div className="neo-pressed rounded-xl p-3 h-full font-mono text-xs leading-relaxed text-on-surface overflow-y-auto whitespace-pre-wrap break-all">
          {activeSession ? (
            lines.length === 0 ? (
              <p className="text-on-surface-variant italic">waiting for output…</p>
            ) : (
              lines.map((l, i) => <div key={i}>{l}</div>)
            )
          ) : (
            <p className="text-on-surface-variant italic">
              Run a command — output streams here live. Agent commands appear as tagged chips.
            </p>
          )}
        </div>
        {evidenceId && (
          <p className="mt-1.5 text-[10px] text-on-surface-variant flex items-center gap-1.5">
            <Icon name="verified_user" size={12} className="text-emerald-600" />
            Captured output persisted as evidence
          </p>
        )}
      </div>
    </div>
  );
}
