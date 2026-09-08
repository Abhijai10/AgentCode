import { useCallback, useEffect, useRef, useState } from "react";
import { daemon } from "./daemon";
import { Icon } from "./Icon";
import type { BrowserPanelResult } from "./types";

// Inbuilt Browser — a real interactive web view (Codex-IDE-style browser
// tab): smart address bar (URLs, hosts, or plain searches), back/forward/
// reload, live CDP screenshots you can CLICK and SCROLL directly (the
// click position maps through the page geometry to a trusted CDP input
// event), keyboard entry into the focused field, and diagnostics.
const HOME_URL = "https://example.com/";
// Live refresh cadence — long enough that the screenshot round-trip
// never queues up on a slow page (the old 1.5s cadence caused visible
// glitchiness when renders took longer than the interval).
const REFRESH_MS = 4000;

type Panel = BrowserPanelResult & {
  geometry?: { scrollX: number; scrollY: number; innerW: number; innerH: number };
  tabs?: string[];
  active_tab?: string;
  tab_count?: number;
};

export function BrowserView() {
  const [tabs, setTabs] = useState<string[]>([]);
  const [activeTab, setActiveTab] = useState<string>("");
  const [input, setInput] = useState(HOME_URL);
  const [currentUrl, setCurrentUrl] = useState("");
  const [img, setImg] = useState<string | null>(null);
  const [panel, setPanel] = useState<Panel | null>(null);
  const [busy, setBusy] = useState(false);
  const [naviging, setNaviging] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [agentViewing, setAgentViewing] = useState<{ label: string; url: string } | null>(null);
  const [bootstrapped, setBootstrapped] = useState(false);
  const [scale, setScale] = useState(1);
  const shotRef = useRef<HTMLImageElement | null>(null);
  const lastPng = useRef<string | null>(null);

  const send = useCallback(
    async (
      action:
        | "navigate"
        | "back"
        | "forward"
        | "reload"
        | "interact"
        | "new_tab"
        | "switch_tab"
        | "close_tab"
        | "list_tabs",
      payload?: string
    ) => {
      setBusy(true);
      const res = await daemon.browserPanel(action, payload ?? "", "desktop");
      setBusy(false);
      if (!res.ok) {
        setError(res.error ?? "browser unavailable");
        return false;
      }
      if (res.panel) {
        setPanel(res.panel as Panel);
        if (Array.isArray((res.panel as Panel).tabs) && (res.panel as Panel).tabs!.length > 0)
          setTabs((res.panel as Panel).tabs!);
        if ((res.panel as Panel).active_tab) setActiveTab((res.panel as Panel).active_tab!);
        setCurrentUrl(res.panel.url);
        if (action === "navigate") setInput(res.panel.url);
        // Skip the image state update when the pixels are identical —
        // avoids a full re-render + image decode per refresh tick.
        if (res.panel.png_base64 && res.panel.png_base64 !== lastPng.current) {
          lastPng.current = res.panel.png_base64;
          setImg(`data:image/jpeg;base64,${res.panel.png_base64}`);
        }
        setError(null);
      }
      return true;
    },
    []
  );

  // First open: land on the start page once.
  useEffect(() => {
    if (bootstrapped) return;
    setBootstrapped(true);
    void send("navigate", HOME_URL);
  }, [bootstrapped]);

  // Live refresh while idle — the page keeps itself current; any user
  // interaction takes over and the next refresh follows it.
  useEffect(() => {
    if (busy || naviging) return;
    const timer = window.setInterval(() => {
      // Pause the live refresh while the window is hidden — no wasted
      // daemon round-trips, no screenshot queue buildup.
      if (document.hidden) return;
      void send("reload");
    }, REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [busy, naviging, send]);

  // Watch-the-agent banner.
  useEffect(() => {
    const tick = async () => {
      const res = await daemon.agentBrowseStatus();
      setAgentViewing(res.status ?? null);
    };
    tick();
    const timer = window.setInterval(tick, 2500);
    return () => window.clearInterval(timer);
  }, []);

  // Map a click on the <img> to CDP page coords using the panel geometry.
  const interactAt = (e: React.MouseEvent) => {
    const el = shotRef.current;
    if (!el || !panel?.geometry) return;
    const rect = el.getBoundingClientRect();
    const px = ((e.clientX - rect.left) / rect.width) * (panel.geometry.innerW || 1440);
    const py = ((e.clientY - rect.top) / rect.height) * (panel.geometry.innerH || 900);
    void send("interact", JSON.stringify({ x: Math.round(px), y: Math.round(py) }));
  };

  // Wheel → CDP wheel scroll at cursor position.
  const onWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const el = shotRef.current;
    if (!el || !panel?.geometry) return;
    const rect = el.getBoundingClientRect();
    const px = ((e.clientX - rect.left) / rect.width) * (panel.geometry.innerW || 1440);
    const py = ((e.clientY - rect.top) / rect.height) * (panel.geometry.innerH || 900);
    void send("interact", JSON.stringify({ dx: 0, dy: Math.round(e.deltaY * 1.5), x: Math.round(px), y: Math.round(py) }));
  };

  // Track rendered scale for the size badge.
  useEffect(() => {
    const el = shotRef.current;
    if (!el) return;
    const update = () => setScale(el.getBoundingClientRect().width / el.naturalWidth);
    update();
    el.addEventListener("load", update);
    window.addEventListener("resize", update);
    return () => {
      el.removeEventListener("load", update);
      window.removeEventListener("resize", update);
    };
  }, [img]);

  const diag = panel?.diagnostics;

  return (
    <main className="flex-1 overflow-hidden flex flex-col bg-surface-container-lowest">
      {/* Compact chrome: nav · address · diagnostics · agent state */}
      <div className="shrink-0 flex items-center gap-1.5 px-3 h-12 border-b border-outline-variant/40 dark:border-white/5 bg-surface/70 backdrop-blur-xl">
        <div className="flex items-center rounded-lg overflow-hidden neo-pressed">
          <button
            onClick={() => void send("back")}
            disabled={!panel}
            title="Back"
            className="p-1.5 text-on-surface-variant hover:text-on-surface hover:bg-surface-variant/30 disabled:opacity-30"
          >
            <Icon name="arrow_back" size={16} />
          </button>
          <button
            onClick={() => void send("forward")}
            disabled={!panel}
            title="Forward"
            className="p-1.5 text-on-surface-variant hover:text-on-surface hover:bg-surface-variant/30 disabled:opacity-30"
          >
            <Icon name="arrow_forward" size={16} />
          </button>
          <button
            onClick={() => void send("reload")}
            disabled={!panel}
            title="Reload"
            className="p-1.5 text-on-surface-variant hover:text-on-surface hover:bg-surface-variant/30 disabled:opacity-30"
          >
            <Icon name="refresh" size={16} className={busy ? "animate-spin" : ""} />
          </button>
        </div>

        <form
          className="flex-1 min-w-0"
          onSubmit={(e) => {
            e.preventDefault();
            setNaviging(true);
            void send("navigate", input.trim()).then(() => setNaviging(false));
          }}
        >
          <div className="flex items-center gap-2 neo-pressed rounded-lg px-2.5 h-8">
            <Icon name="lock" size={12} className="text-emerald-500 shrink-0" />
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder="Search or enter address"
              spellCheck={false}
              className="flex-1 min-w-0 bg-transparent text-[13px] text-on-surface font-mono outline-none placeholder:text-on-surface-variant/50"
            />
            {naviging && <Icon name="autorenew" size={13} className="animate-spin text-primary shrink-0" />}
          </div>
        </form>

        {diag && (diag.console_errors.length > 0 || diag.page_errors.length > 0 || diag.network_failures.length > 0) && (
          <button
            title={`${diag.console_errors.length} console · ${diag.page_errors.length} page · ${diag.network_failures.length} network`}
            className="shrink-0 flex items-center gap-1 text-[10px] px-2 py-1 rounded-full bg-amber-500/10 text-amber-600 dark:text-amber-400"
          >
            <Icon name="warning" size={11} />
            {diag.console_errors.length + diag.page_errors.length + diag.network_failures.length}
          </button>
        )}
        {agentViewing && (
          <span className="shrink-0 flex items-center gap-1.5 text-[10px] text-primary bg-primary/10 px-2 py-1 rounded-full">
            <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
            agent viewing
          </span>
        )}
      </div>

      {/* Tab strip — multiple tabs in the ONE shared browser, like a
          normal browser: click to switch, + to open, × to close. */}
      <div className="shrink-0 flex items-center gap-1 px-2 h-9 bg-surface-container-low border-b border-outline-variant/30 dark:border-white/5 overflow-x-auto no-scrollbar">
        {tabs.map((tab, i) => (
          <span
            key={tab}
            className={`shrink-0 flex items-center gap-1.5 pl-2.5 pr-1.5 h-7 rounded-t-lg text-[11px] font-medium group ${
              activeTab === tab
                ? "bg-surface text-on-surface shadow-sm"
                : "text-on-surface-variant hover:bg-surface-variant/30"
            }`}
          >
            <button
              onClick={() => void send("switch_tab", tab)}
              title={`Tab ${i + 1} — ${tab === activeTab ? panel?.url ?? "" : "switch to this tab"}`}
              className="outline-none"
            >
              Tab {i + 1}
              {activeTab === tab && panel?.title ? ` — ${panel.title.slice(0, 18)}` : ""}
            </button>
            <button
              onClick={() => void send("close_tab", tab)}
              className="opacity-40 group-hover:opacity-100 hover:text-red-500"
              title="Close this tab"
            >
              <Icon name="close" size={11} />
            </button>
          </span>
        ))}
        <button
          onClick={() => void send("new_tab")}
          className="shrink-0 w-7 h-7 rounded-lg flex items-center justify-center text-on-surface-variant hover:text-on-surface hover:bg-surface-variant/30"
          title="Open a new tab"
        >
          <Icon name="add" size={14} />
        </button>
      </div>

      {/* Agent banner */}
      {agentViewing && (
        <div className="shrink-0 px-3 py-1.5 text-[11px] text-primary bg-primary/5 flex items-center gap-2">
          <Icon name="smart_toy" size={12} />
          Agent viewing: {agentViewing.label} — design runs share this browser; your page returns when the run finishes.
        </div>
      )}

      {error && (
        <div className="shrink-0 flex items-center gap-2 px-4 py-2 text-xs text-red-600 dark:text-red-400 bg-red-500/5">
          <Icon name="error" size={14} fill /> {error}
        </div>
      )}

      {/* Interactive page viewport */}
      <div className="flex-1 overflow-auto p-3 flex items-start justify-center browser-canvas">
        {img ? (
          <figure className="relative inline-block rounded-xl overflow-hidden shadow-2xl ring-1 ring-black/10 dark:ring-white/10 cursor-pointer select-none">
            <img
              ref={shotRef}
              src={img}
              alt={panel?.title || "page"}
              onClick={interactAt}
              onWheel={onWheel}
              tabIndex={0}
              onKeyDown={(e) => {
                // Route printable keys + navigational keys into the page.
                const routed = ["Enter", "Tab", "Backspace", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Escape"];
                if (e.key.length === 1 || routed.includes(e.key)) {
                  e.preventDefault();
                  void send("interact", JSON.stringify({ key: e.key }));
                }
              }}
              className="block max-w-full"
              draggable={false}
            />
          </figure>
        ) : busy || naviging ? (
          <div className="flex items-center gap-2 text-sm text-on-surface-variant mt-16">
            <Icon name="autorenew" size={18} className="animate-spin" />
            Loading…
          </div>
        ) : (
          <div className="text-center space-y-3 text-on-surface-variant mt-16">
            <Icon name="language" size={40} className="text-primary" />
            <p className="text-sm">Search the web or enter an address above.</p>
          </div>
        )}
      </div>

      {/* Status strip */}
      <div className="shrink-0 flex items-center gap-3 px-3 h-7 border-t border-outline-variant/40 dark:border-white/5 text-[10px] text-on-surface-variant bg-surface/70">
        <span className="font-mono truncate max-w-[45%]">{currentUrl || "—"}</span>
        {panel?.diagnostics && (
          <span className="text-emerald-600 dark:text-emerald-400">HTTP {String(panel.diagnostics.http_status ?? "—")}</span>
        )}
        {scale < 1 && <span>{Math.round(scale * 100)}%</span>}
        <span className="ml-auto">{panel ? "click, scroll & type on the page" : ""}</span>
      </div>
    </main>
  );
}
