import { useCallback, useEffect, useRef, useState } from "react";
import { daemon } from "./daemon";
import { Icon } from "./Icon";

// Inbuilt Browser — a real full-page web view (Codex-IDE-style browser tab),
// driven by the daemon's ONE persistent headless Chrome through CDP:
// navigate any http(s) URL, back/forward/reload via history traversal,
// framed live screenshots, console/network diagnostics.  The engine is the
// SAME shared runtime design runs use (watch-the-agent): while a run is
// active the header shows what the agent is viewing.
const HOME_URL = "https://example.com/";
const POLL_MS = 1200;

import type { BrowserPanelResult } from "./types";

type PanelMeta = BrowserPanelResult;

export function BrowserView() {
  const [urlInput, setUrlInput] = useState(HOME_URL);
  const [currentUrl, setCurrentUrl] = useState("");
  const [img, setImg] = useState<string | null>(null);
  const [meta, setMeta] = useState<PanelMeta | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [live, setLive] = useState(false);
  const [agentViewing, setAgentViewing] = useState<{ label: string; url: string } | null>(null);
  const [bootstrapped, setBootstrapped] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const act = useCallback(
    async (action: "navigate" | "back" | "forward" | "reload", url?: string) => {
      setBusy(true);
      setError(null);
      const res = await daemon.browserPanel(action, url, "desktop");
      setBusy(false);
      if (!res.ok) {
        setError(res.error ?? "browser unavailable");
        return;
      }
      if (res.panel) {
        setMeta(res.panel);
        setCurrentUrl(res.panel.url);
        setUrlInput(res.panel.url);
        if (res.panel.png_base64) setImg(`data:image/png;base64,${res.panel.png_base64}`);
      }
    },
    []
  );

  // First open: navigate to the start page once.
  useEffect(() => {
    if (bootstrapped) return;
    setBootstrapped(true);
    void act("navigate", HOME_URL);
  }, [bootstrapped]);

  // Live mode: auto-refresh the page screenshot like a real browser view.
  useEffect(() => {
    if (!live || busy) return;
    const timer = window.setInterval(() => void act("reload"), POLL_MS);
    return () => window.clearInterval(timer);
  }, [live, busy, act]);

  // Watch-the-agent: surface what a design run is viewing while it works.
  useEffect(() => {
    const tick = async () => {
      const res = await daemon.agentBrowseStatus();
      setAgentViewing(res.status ?? null);
    };
    tick();
    const timer = window.setInterval(tick, 1000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <main className="flex-1 overflow-hidden flex flex-col bg-surface-container-lowest">
      {/* URL bar */}
      <div className="shrink-0 flex items-center gap-2 px-4 py-3 border-b border-outline-variant/40 dark:border-white/5 bg-surface/80 backdrop-blur-md">
        <button
          onClick={() => void act("back")}
          disabled={!meta}
          title="Back"
          className="p-2 rounded-lg text-on-surface-variant hover:bg-surface-variant/40 dark:hover:bg-white/5 disabled:opacity-30"
        >
          <Icon name="arrow_back" size={18} />
        </button>
        <button
          onClick={() => void act("forward")}
          disabled={!meta}
          title="Forward"
          className="p-2 rounded-lg text-on-surface-variant hover:bg-surface-variant/40 dark:hover:bg-white/5 disabled:opacity-30"
        >
          <Icon name="arrow_forward" size={18} />
        </button>
        <button
          onClick={() => void act("reload")}
          disabled={!meta}
          title="Reload"
          className="p-2 rounded-lg text-on-surface-variant hover:bg-surface-variant/40 dark:hover:bg-white/5 disabled:opacity-30"
        >
          <Icon name="refresh" size={18} />
        </button>
        <form
          className="flex-1 flex items-center gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void act("navigate", urlInput.trim());
          }}
        >
          <div className="flex-1 neo-pressed rounded-xl px-3 py-2 flex items-center gap-2">
            <Icon name="lock" size={14} className="text-emerald-600 shrink-0" />
            <input
              ref={inputRef}
              value={urlInput}
              onChange={(e) => setUrlInput(e.target.value)}
              placeholder="https://…"
              spellCheck={false}
              className="flex-1 bg-transparent text-sm text-on-surface font-mono outline-none"
            />
          </div>
          <button
            type="submit"
            disabled={busy}
            className="neo-button px-4 py-2 rounded-xl text-sm font-medium text-primary flex items-center gap-1.5 disabled:opacity-50"
          >
            {busy ? <Icon name="autorenew" size={16} className="animate-spin" /> : <Icon name="travel_explore" size={16} />}
            Go
          </button>
        </form>
        <button
          onClick={() => setLive((v) => !v)}
          title={live ? "Stop live refresh" : "Refresh this page automatically"}
          aria-pressed={live}
          className={`p-2 rounded-lg flex items-center gap-1 text-xs font-medium ${
            live ? "text-primary bg-primary/10" : "text-on-surface-variant hover:bg-surface-variant/40 dark:hover:bg-white/5"
          }`}
        >
          <Icon name={live ? "pause_circle" : "play_circle"} size={16} />
          {live ? "Live on" : "Live off"}
        </button>
      </div>

      {/* Agent banner */}
      {agentViewing && (
        <div className="shrink-0 flex items-center gap-2 px-4 py-1.5 text-[11px] text-primary bg-primary/5">
          <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
          Agent viewing: {agentViewing.label} — design runs share this browser engine; your page returns when the run finishes.
        </div>
      )}

      {error && (
        <div className="shrink-0 flex items-center gap-2 px-4 py-2 text-xs text-red-600 dark:text-red-400 bg-red-500/5">
          <Icon name="error" size={14} fill />
          {error}
        </div>
      )}

      {/* Page viewport */}
      <div className="flex-1 overflow-auto p-4 flex items-start justify-center">
        {img ? (
          <figure className="relative inline-block shadow-lg rounded-xl overflow-hidden ring-1 ring-black/5 dark:ring-white/10">
            <img src={img} alt={meta?.title || "page"} className="block max-w-full" />
          </figure>
        ) : busy ? (
          <div className="flex items-center gap-2 text-sm text-on-surface-variant">
            <Icon name="autorenew" size={18} className="animate-spin" />
            Loading…
          </div>
        ) : (
          <div className="text-center space-y-3 text-on-surface-variant mt-16">
            <Icon name="language" size={40} className="text-primary" />
            <p className="text-sm">Enter a URL above to start browsing.</p>
          </div>
        )}
      </div>

      {/* Status bar */}
      <div className="shrink-0 flex items-center gap-4 px-4 py-2 border-t border-outline-variant/40 dark:border-white/5 text-[11px] text-on-surface-variant bg-surface/80">
        <span className="font-mono truncate max-w-[40%]">{currentUrl || "—"}</span>
        {meta?.diagnostics && (
          <>
            <span className={meta.diagnostics.console_errors.length ? "text-amber-600 dark:text-amber-400" : ""}>
              {meta.diagnostics.console_errors.length} console
            </span>
            <span className={meta.diagnostics.page_errors.length ? "text-red-600 dark:text-red-400" : ""}>
              {meta.diagnostics.page_errors.length} page errors
            </span>
            <span className={meta.diagnostics.network_failures.length ? "text-red-600 dark:text-red-400" : ""}>
              {meta.diagnostics.network_failures.length} network
            </span>
            {meta.diagnostics.http_status !== undefined && <span>HTTP {String(meta.diagnostics.http_status)}</span>}
          </>
        )}
        <span className="ml-auto">
          {meta?.viewport ? `${meta.viewport.width}×${meta.viewport.height}` : ""}
        </span>
      </div>
    </main>
  );
}
