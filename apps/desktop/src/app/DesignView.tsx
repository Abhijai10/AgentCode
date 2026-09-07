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
  ProductAnalysis,
  ReferenceAnalysis,
  DesignBrief,
  DesignGrammar,
  DesignState,
  DesignCritique,
  DesignRepair,
  DesignMemory,
  DesignBrowserResult,
  BrowserPanelResult,
  DesignQaReport,
  VisualCritique,
  DesignConstraint,
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

function severityColor(severity: number): string {
  if (severity >= 3) return "text-red-600 dark:text-red-400";
  if (severity === 2) return "text-amber-600 dark:text-amber-400";
  return "text-on-surface-variant";
}

function listTags(value: string[] | undefined): JSX.Element | null {
  if (!value || value.length === 0) return null;
  return (
    <ul className="space-y-1">
      {value.map((item, i) => (
        <li key={i} className="text-xs text-on-surface-variant flex gap-1.5">
          <span className="text-primary mt-0.5">·</span>
          <span>{item}</span>
        </li>
      ))}
    </ul>
  );
}

export function DesignView({
  project,
  onOpenMission,
  daemonConnected = false,
  browserOpen: browserOpenProp,
  onBrowserOpenChange,
  ...props
}: {
  project: Project | null;
  missionId: string | null;
  daemonConnected?: boolean;
  /** Sidebar rail control: the inbuilt-browser panel's open state is lifted
   *  so the global browser toggle (watch-the-agent) can drive it. */
  browserOpen?: boolean;
  onBrowserOpenChange?(open: boolean): void;
  onOpenMission(missionId: string): void;
}) {
  void props.missionId;
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [designListCollapsed, setDesignListCollapsed] = useState(false);
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

  // Right-panel design state
  const [analysis, setAnalysis] = useState<ProductAnalysis | null>(null);
  const [reference, setReference] = useState<ReferenceAnalysis | null>(null);
  const [brief, setBrief] = useState<DesignBrief | null>(null);
  const [grammar, setGrammar] = useState<DesignGrammar | null>(null);
  const [designState, setDesignState] = useState<DesignState | null>(null);
  const [critique, setCritique] = useState<DesignCritique | null>(null);
  const [repair, setRepair] = useState<DesignRepair | null>(null);
  const [browser, setBrowser] = useState<DesignBrowserResult | null>(null);

  const [visualCritique, setVisualCritique] = useState<VisualCritique | null>(null);
  const [constraints, setConstraints] = useState<DesignConstraint[]>([]);
  const [constraintDraft, setConstraintDraft] = useState("");
  const [qa, setQa] = useState<{
    responsive?: DesignQaReport;
    accessibility?: DesignQaReport;
    functional?: DesignQaReport;
  }>({});
  // F2: the design contract (executable specification) + standalone QA runs.
  const [contract, setContract] = useState<Record<string, unknown> | null>(null);
  const [standaloneQa, setStandaloneQa] = useState<DesignQaReport | null>(null);
  // Stitch-parity: generative mockups (prompt -> variants -> winner).
  const [mockupPrompt, setMockupPrompt] = useState("");
  // Multi-screen flow (Stitch parity): one prompt per line = one screen.
  const [flowScreens, setFlowScreens] = useState("");
  const [flow, setFlow] = useState<{
    flow_id?: string;
    screen_count?: number;
    failed_screens?: { index: number; screen: string; error: string }[];
    screens?: { index: number; screen: string; winner?: number; winner_html?: string; run?: Record<string, unknown> }[];
  } | null>(null);
  const [mockups, setMockups] = useState<{
    variants: { variant: number; layout: string; palette_intent: string; html: string; score?: Record<string, unknown> }[];
    winner: number;
    spec_parse_failed?: boolean;
  } | null>(null);
  const [exportTarget, setExportTarget] = useState("src/pages/GeneratedLanding.tsx");
  const [exportResult, setExportResult] = useState<{ missionId: string; target: string } | null>(null);
  const [viewportHint, setViewportHint] = useState("desktop");
  // ── Inbuilt browser panel (Codex-style): one persistent Chrome, framed
  //    screenshots, URL bar, history. Lives in DesignView because the panel
  //    exists to browse the live preview; it navigates any http(s) URL.
  //    Open state is lifted to the app shell when the sidebar rail drives
  //    it (watch-the-agent toggle); the in-view button uses the same truth.
  const [panelOpenLocal, setPanelOpenLocal] = useState(false);
  const panelOpen = browserOpenProp ?? panelOpenLocal;
  const setPanelOpen = (open: boolean | ((prev: boolean) => boolean)) => {
    const next = typeof open === "function" ? open(panelOpen) : open;
    setPanelOpenLocal(next);
    onBrowserOpenChange?.(next);
  };
  const [panelUrl, setPanelUrl] = useState("");
  const [panelImg, setPanelImg] = useState<string | null>(null);
  const [panelMeta, setPanelMeta] = useState<BrowserPanelResult | null>(null);
  const [panelBusy, setPanelBusy] = useState(false);
  // Watch-the-agent: the design run's live observation point, shown in the
  // browser panel while a run is active.
  const [agentViewing, setAgentViewing] = useState<{ label: string; url: string } | null>(null);
  const [panelError, setPanelError] = useState<string | null>(null);

  // Poll the agent's live browse status while the panel is open.
  useEffect(() => {
    if (!panelOpen) return;
    let stop = false;
    const tick = async () => {
      const res = await daemon.agentBrowseStatus();
      if (!stop) setAgentViewing(res.status ?? null);
    };
    tick();
    const timer = setInterval(tick, 1000);
    return () => {
      stop = true;
      clearInterval(timer);
    };
  }, [panelOpen]);

  const panelNavigate = useCallback(
    async (action: "navigate" | "back" | "forward" | "reload" | "close", url?: string) => {
      if (action !== "close" && !panelOpen) setPanelOpen(true);
      setPanelBusy(true);
      setPanelError(null);
      const res = await daemon.browserPanel(action, url, viewportHint);
      setPanelBusy(false);
      if (!res.ok) {
        setPanelError(res.error ?? "browser panel unavailable");
        setPanelImg(null);
        return;
      }
      if (action === "close") {
        setPanelOpen(false);
        setPanelImg(null);
        setPanelMeta(null);
        return;
      }
      const panel = res.panel;
      if (panel) {
        setPanelMeta(panel);
        setPanelUrl(panel.url);
        if (panel.png_base64) setPanelImg(`data:image/png;base64,${panel.png_base64}`);
      }
    },
    [panelOpen, viewportHint]
  );
  const [rightOpen, setRightOpen] = useState(true);

  const messagesEndRef = useRef<HTMLDivElement>(null);

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
      const designConvs = convs.filter((c) => c.mode === "DESIGN");
      setConversations(designConvs);
      if (!activeConvId && designConvs.length > 0) {
        setActiveConvId(designConvs[0].id);
      }
      setStatus(designConvs.length > 0 ? "loaded" : "no_chat");
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

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [convDetail?.messages?.length]);

  const refreshConversations = useCallback(async () => {
    if (!project) return;
    const convs = await daemon.listConversations(project.path);
    setConversations(convs.filter((c) => c.mode === "DESIGN"));
  }, [project]);

  const refreshActive = useCallback(async () => {
    if (!activeConvId) return;
    const detail = await daemon.getConversation(activeConvId);
    if (detail) setConvDetail(detail);
    // Project-scoped durable constraints (design memory across chats).
    const consRes = await daemon.designConstraintsGet(activeConvId);
    if (consRes.ok && consRes.constraints) {
      setConstraints(consRes.constraints.constraints ?? []);
    }
  }, [activeConvId]);

  const handleNewChat = async () => {
    if (!project) return;
    setError(null);
    const result = await daemon.createConversation(project.path, "DESIGN", "New Design Chat");
    if (result.ok && result.conversation_id) {
      setActiveConvId(result.conversation_id);
      await refreshConversations();
      setStatus("loaded");
    } else {
      setError(result.error || "Could not create design chat");
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
    const replyResult = await daemon.designSend(
      activeConvId,
      trimmed,
      pendingAttachments.map((a) => a.id)
    );
    if (replyResult.ok) {
      setPendingAttachments([]);
      await refreshActive();
    } else {
      setError(replyResult.error || "Could not get design response");
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

  // ── Design panel actions ──────────────────────────────────────────────

  const runPanel = async (fn: () => Promise<void>) => {
    if (!activeConvId || panelBusy) return;
    setPanelBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPanelBusy(false);
    }
  };

  const handleUnderstand = () =>
    runPanel(async () => {
      const r = await daemon.designUnderstand(activeConvId!);
      if (r.ok && r.analysis) setAnalysis(r.analysis);
      else setError(r.error || "Could not analyze project");
    });

  const handleAnalyzeReference = () =>
    runPanel(async () => {
      // Analyze the most recent image attachment in this conversation.  The
      // daemon refuses non-image attachments with a clear error.
      const images = (convDetail?.attachments ?? []).filter((a) =>
        a.mime_type.startsWith("image/")
      );
      if (images.length === 0) {
        setError("Attach a reference image first, then run reference analysis.");
        return;
      }
      const latest = images[images.length - 1];
      const r = await daemon.designAnalyzeReference(activeConvId!, latest.id);
      if (r.ok && r.analysis) {
        setReference(r.analysis);
        await refreshActive();
      } else {
        setError(r.error || "Could not analyze reference image");
      }
    });

  const handleBrief = () =>
    runPanel(async () => {
      const r = await daemon.designBrief(activeConvId!, "target users", "primary workflow");
      if (r.ok && r.brief) setBrief(r.brief);
      else setError(r.error || "Could not create design brief");
    });

  const handleGrammar = () =>
    runPanel(async () => {
      const r = await daemon.designGrammar(activeConvId!);
      if (r.ok && r.grammar) setGrammar(r.grammar);
      else setError(r.error || "Could not create design grammar");
    });

  const handleDesignState = () =>
    runPanel(async () => {
      const r = await daemon.designState(activeConvId!);
      if (r.ok && r.state) setDesignState(r.state);
      else setError(r.error || "Could not generate design state");
    });

  const handleCritique = () =>
    runPanel(async () => {
      const domText = browser?.visible_text ?? "";
      const r = await daemon.designCritique(
        activeConvId!,
        domText || brief?.product || "implementation",
        "rendered"
      );
      if (r.ok && r.critique) setCritique(r.critique);
      else setError(r.error || "Could not critique design");
    });

  const [designMemory, setDesignMemory] = useState<DesignMemory | null>(null);
  const [iterationHistory, setIterationHistory] = useState<
    { iteration: number; passed: boolean; remaining_issues?: string[] }[] | null
  >(null);

  const handleDesignMemory = () =>
    void (async () => {
      if (!activeConvId) return;
      setPanelBusy(true);
      try {
        const m = await daemon.designMemoryGet(activeConvId);
        if (m.ok && m.memory) setDesignMemory(m.memory as DesignMemory);
      } finally {
        setPanelBusy(false);
      }
    })();

  const handleIterationHistory = () =>
    void (async () => {
      if (!activeConvId) return;
      setPanelBusy(true);
      try {
        const h = await daemon.designIterations(activeConvId);
        if (h.ok && h.iterations) {
          const doc = h.iterations as { iterations?: unknown[] };
          setIterationHistory(
            (doc.iterations ?? []).map(
              (it) => it as { iteration: number; passed: boolean; remaining_issues?: string[] }
            )
          );
        }
      } finally {
        setPanelBusy(false);
      }
    })();

  const handleRepair = () =>
    runPanel(async () => {
      const domText = browser?.visible_text ?? "";
      const r = await daemon.designRepair(activeConvId!, domText || "implementation", "rendered");
      if (r.ok && r.repair) setRepair(r.repair);
      else setError(r.error || "Could not generate repair plan");
    });

  const handleBrowser = () =>
    runPanel(async () => {
      const r = await daemon.designBrowser(activeConvId!, undefined, undefined, undefined, viewportHint);
      if (r.ok && r.browser) {
        setBrowser(r.browser);
        // Real QA: run the layered report against the same real browser
        // target the user just inspected (the preview URL), not the DOM
        // text.  The daemon measures real layout via CDP.
        const qaResult = await daemon.designQaReport(
          activeConvId!,
          r.browser.url,
          undefined,
          false,
          viewportHint
        );
        if (qaResult.ok && qaResult.qa) {
          const layers = qaResult.qa as unknown as {
            layers?: {
              responsive?: DesignQaReport;
              accessibility?: DesignQaReport;
              functional?: DesignQaReport;
            };
          };
          setQa({
            responsive: layers.layers?.responsive,
            accessibility: layers.layers?.accessibility,
            functional: layers.layers?.functional,
          });
        }
      } else {
        setError(r.error || "Could not inspect browser");
      }
    });

  const handlePreview = () =>
    runPanel(async () => {
      const r = await daemon.designPreviewStart(activeConvId!);
      if (!r.ok) setError(r.error || "Could not start preview");
    });

  // Stitch-parity: generate candidate mockup variants from a prompt.  The
  // model fills a constrained spec; the daemon renders real variants and
  // scores them in the real browser; the winner is highlighted here.
  const handleGenerateMockups = () =>
    runPanel(async () => {
      if (!mockupPrompt.trim()) {
        setError("Describe the screen you want (e.g. 'landing page for the terminal app').");
        return;
      }
      const r = await daemon.designGenerateMockups(activeConvId!, mockupPrompt, 3, false);
      if (r.ok && r.variants) {
        setMockups({ variants: r.variants, winner: r.winner ?? 0, spec_parse_failed: r.spec_parse_failed });
      } else {
        setError(r.error || "Could not generate mockups");
      }
    });

  // Multi-screen Stitch flow: generate a mockup run per screen prompt
  // (one per line) and group the winners into one flow document — the
  // app-skeleton step toward full Stitch parity.
  const handleGenerateFlow = () =>
    runPanel(async () => {
      const screens = flowScreens
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line.length > 0);
      if (screens.length === 0) {
        setError("Enter one screen prompt per line (e.g. 'landing', 'pricing', 'docs').");
        return;
      }
      const r = await daemon.designGenerateFlow(activeConvId!, screens, 2, false);
      if (r.ok && r.screens) {
        setFlow({ flow_id: r.flow_id, screen_count: r.screen_count, failed_screens: r.failed_screens, screens: r.screens });
      } else {
        setError(r.error || "Could not generate the flow");
      }
    });

  // Stitch-parity: export the winner as a real React component through a
  // governed mission (Tool Broker writes + ChangeSets + verification).
  const handleExportWinner = () =>
    runPanel(async () => {
      const r = await daemon.designExportWinner(activeConvId!, exportTarget);
      if (r.ok && r.missionId) {
        setExportResult({ missionId: r.missionId, target: r.target ?? exportTarget });
      } else {
        setError(r.error || "Could not export the winner");
      }
    });

  // F2: the full design contract — the executable specification of this
  // design conversation (tokens, layout rules, constraints) as the mission
  // would receive it.
  const handleContract = () =>
    runPanel(async () => {
      const r = await daemon.designContractGet(activeConvId!);
      if (r.ok && r.contract) setContract(r.contract);
      else setError(r.error || "Could not build design contract");
    });

  // F2: standalone per-kind QA runs (the layered report runs with Inspect;
  // these run one kind deeply on the live preview URL or content).
  const handleStandaloneQa = (kind: "responsive" | "accessibility" | "functional") =>
    runPanel(async () => {
      const url = browser?.url;
      const r = await daemon.designQa(activeConvId!, kind, undefined, url, undefined, false, viewportHint);
      if (r.ok && r.qa) setStandaloneQa(r.qa);
      else setError(r.error || `Could not run ${kind} QA`);
    });

  // ── Render helpers ────────────────────────────────────────────────────

  function renderMessage(msg: Message) {
    const isUser = msg.role === "user";
    const isAssistant = msg.role === "assistant";
    let providerModel: { provider_id?: string; model_name?: string } | null = null;
    if (isAssistant && msg.metadata) {
      try {
        const meta =
          typeof msg.metadata === "string" ? JSON.parse(msg.metadata) : msg.metadata;
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
            {isAssistant && providerModel && (
              <div className="mt-2 flex items-center gap-2 text-[10px] text-on-surface-variant">
                <Icon name="smart_toy" size={12} />
                <span>
                  {providerModel.model_name
                    ? providerModel.model_name
                    : providerModel.provider_id || "AI response"}
                </span>
              </div>
            )}
            {msg.mission_ref && (
              <div className="mt-2 flex items-center gap-2 text-xs text-on-surface-variant">
                <Icon name="terminal" size={12} />
                <span>Mission: {msg.mission_ref}</span>
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

  // Inbuilt browser panel (watch-the-agent): extracted so it renders in
  // the loaded view AND the no-project state — the sidebar rail toggle
  // always has a real surface.
  const browserPanelSection = (
      <section className="neo-raised rounded-2xl p-4">
        <div className="flex items-center justify-between mb-2">
          <h4 className="text-sm font-semibold text-on-surface">Inbuilt Browser</h4>
          {agentViewing && (
            <div className="flex items-center gap-1.5 text-[10px] text-primary bg-primary/5 rounded-full px-2 py-0.5">
              <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
              Agent viewing: {agentViewing.label}
            </div>
          )}
          <button
            onClick={() => (panelOpen ? panelNavigate("close") : setPanelOpen(true))}
            className="text-[10px] text-primary hover:opacity-80"
          >
            {panelOpen ? "Close browser" : "Open browser"}
          </button>
        </div>
        {panelOpen ? (
          <div className="space-y-2">
            {/* URL bar + controls — familiar browser chrome, no reinvention */}
            <div className="flex items-center gap-1">
              <button
                onClick={() => panelNavigate("back")}
                disabled={panelBusy}
                title="Back"
                aria-label="Back"
                className="neo-btn rounded-lg px-2 py-1 text-[11px] text-on-surface-variant disabled:opacity-50"
              >
                ←
              </button>
              <button
                onClick={() => panelNavigate("forward")}
                disabled={panelBusy}
                title="Forward"
                aria-label="Forward"
                className="neo-btn rounded-lg px-2 py-1 text-[11px] text-on-surface-variant disabled:opacity-50"
              >
                →
              </button>
              <button
                onClick={() => panelNavigate("reload")}
                disabled={panelBusy}
                title="Reload"
                aria-label="Reload"
                className="neo-btn rounded-lg px-2 py-1 text-[11px] text-on-surface-variant disabled:opacity-50"
              >
                ⟳
              </button>
              <form
                className="flex-1 flex items-center gap-1"
                onSubmit={(e) => {
                  e.preventDefault();
                  panelNavigate("navigate", panelUrl);
                }}
              >
                <input
                  value={panelUrl}
                  onChange={(e) => setPanelUrl(e.target.value)}
                  placeholder="http://127.0.0.1:5173"
                  aria-label="Browser address"
                  className="neo-input flex-1 rounded-lg px-2 py-1 text-[11px] font-mono bg-transparent text-on-surface"
                />
                <button
                  type="submit"
                  disabled={panelBusy}
                  className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
                >
                  Go
                </button>
              </form>
            </div>
            {panelBusy && (
              <p className="text-[10px] text-on-surface-variant">Loading…</p>
            )}
            {panelError && (
              <p className="text-[11px] text-red-600 dark:text-red-400" role="alert">
                {panelError}
              </p>
            )}
            {/* The live page: framed screenshot from the persistent Chrome */}
            {panelImg ? (
              <div className="rounded-xl overflow-hidden border border-outline-variant bg-white">
                <img
                  src={panelImg}
                  alt={`Browser view of ${panelMeta?.url ?? panelUrl}`}
                  className="w-full h-auto block"
                />
              </div>
            ) : !panelBusy && !panelError ? (
              <p className="text-xs text-on-surface-variant">
                Enter a URL and press Go — AgentCode keeps one Chrome alive for the
                panel and renders the live page here.
              </p>
            ) : null}
            {panelMeta && (
              <p className="text-[10px] text-on-surface-variant">
                HTTP {panelMeta.diagnostics.http_status} · console errors{" "}
                {panelMeta.diagnostics.console_errors.length} · network failures{" "}
                {panelMeta.diagnostics.network_failures.length}
                {panelMeta.viewport
                  ? ` · ${panelMeta.viewport.width}×${panelMeta.viewport.height}`
                  : ""}
              </p>
            )}
          </div>
        ) : (
          <p className="text-xs text-on-surface-variant">
            An embedded browser like Codex: one persistent Chrome, address bar,
            back/forward, live screenshot rendering with console + network diagnostics.
          </p>
        )}
      </section>
  );

  if (status === "no_project") {
    return (
      <div className="flex-1 flex overflow-hidden">
        <main className="flex-1 flex items-center justify-center">
          <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
            <Icon name="design_services" size={40} className="text-primary mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-on-surface mb-2">Open a Project</h3>
            <p className="text-sm text-on-surface-variant">
              Select a project to design, preview, critique, and verify its interface.
            </p>
          </div>
        </main>
        {/* Watch-the-agent: the inbuilt browser is standalone (any http(s)
            URL) — the sidebar rail toggle always has a real surface, even
            before a project is opened. */}
        {panelOpen && (
          <aside className="w-96 shrink-0 border-l border-outline-variant/40 dark:border-white/5 bg-surface/50 overflow-y-auto p-4">
            {browserPanelSection}
            <p className="mt-3 text-[10px] text-on-surface-variant px-1">
              Open a project to unlock design conversations, previews, and QA — the browser
              itself works anywhere.
            </p>
          </aside>
        )}
      </div>
    );
  }

  if (status === "daemon_unavailable") {
    return (
      <main className="flex-1 flex items-center justify-center">
        <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
          <Icon name="dns_off" size={40} className="text-red-500 mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-on-surface mb-2">Daemon Unavailable</h3>
          <p className="text-sm text-on-surface-variant">
            The AgentCode daemon is not responding. Design conversations are persisted and will
            be available when the daemon restarts.
          </p>
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 flex overflow-hidden">
      {/* Conversation List */}
      {designListCollapsed ? (
        <button
          onClick={() => setDesignListCollapsed(false)}
          title="Show design chats"
          className="shrink-0 w-8 border-r border-outline-variant/40 dark:border-white/5 bg-surface/50 flex items-center justify-center text-on-surface-variant hover:text-primary"
        >
          <Icon name="arrow_forward" size={16} />
        </button>
      ) : (
        <aside className="w-60 shrink-0 flex flex-col border-r border-outline-variant/40 dark:border-white/5 bg-surface/50 relative">
          <button
            onClick={() => setDesignListCollapsed(true)}
            title="Collapse design chats"
            className="absolute top-2 right-2 z-10 p-1 rounded-lg text-on-surface-variant hover:bg-surface-variant/40 dark:hover:bg-white/5"
          >
            <Icon name="arrow_back" size={14} className="rotate-180" />
          </button>
        <div className="p-3 border-b border-outline-variant/40 dark:border-white/5">
          <button
            onClick={handleNewChat}
            className="w-full neo-button rounded-xl py-2.5 flex items-center justify-center gap-2 text-sm font-medium text-primary"
          >
            <Icon name="add" size={18} />
            New Design Chat
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
              No design chats yet. Start a new design chat.
            </div>
          )}
        </div>
      </aside>
      )}

      {/* Chat Area */}
      <div className="flex-1 flex flex-col overflow-hidden min-w-0">
        {!activeConvId ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center neo-pressed rounded-2xl p-8 max-w-sm">
              <Icon name="design_services" size={40} className="text-primary mx-auto mb-4" />
              <h3 className="text-lg font-semibold text-on-surface mb-2">Select a Design Chat</h3>
              <p className="text-sm text-on-surface-variant">
                Choose a design chat from the sidebar or start a new one.
              </p>
            </div>
          </div>
        ) : (
          <>
            <div className="shrink-0 px-6 py-3 border-b border-outline-variant/40 dark:border-white/5 flex items-center justify-between">
              <div className="flex items-center gap-3 min-w-0">
                <div className="min-w-0">
                  <h2 className="text-lg font-semibold text-on-surface truncate">
                    {convDetail?.title || "Loading..."}
                  </h2>
                  {convDetail && (
                    <span className="text-xs font-bold uppercase text-primary">Design Studio</span>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => setRightOpen((v) => !v)}
                  className="neo-button rounded-lg px-3 py-1.5 text-xs font-medium text-on-surface-variant flex items-center gap-1.5"
                  title="Toggle design context panel"
                  aria-label="Toggle design context panel"
                >
                  <Icon name={rightOpen ? "panel_close" : "panel_open"} size={16} />
                  {rightOpen ? "Hide Context" : "Show Context"}
                </button>
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
                  Describe the interface you want to design — its purpose, users, and feel.
                  AgentCode will understand the project, draft a brief and grammar, then
                  implement, preview, critique, and repair.
                </div>
              )}
              <div ref={messagesEndRef} />
            </div>

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
                          {a.size_bytes > 1024 ? `${(a.size_bytes / 1024).toFixed(0)}KB` : `${a.size_bytes}B`}
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

            <div className="shrink-0 px-6 py-4 border-t border-outline-variant/40 dark:border-white/5">
              {error && (
                <div className="flex items-center gap-2 text-xs text-red-600 dark:text-red-400 mb-2">
                  <Icon name="error" size={14} fill /> {error}
                </div>
              )}
              <div className="neo-raised rounded-[20px] p-2">
                <div className="neo-pressed rounded-[16px] px-4 py-3 flex items-end gap-2">
                  <textarea
                    className="flex-1 bg-transparent border-none outline-none resize-none text-sm text-on-surface placeholder:text-on-surface-variant/50 max-h-[120px] focus:ring-0 p-0"
                    placeholder='Describe what to design, e.g. "a technical, quiet, premium dashboard"…'
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
                    title="Attach reference image or spec"
                    aria-label="Attach file"
                  >
                    <Icon name="attach_file" size={18} />
                  </button>
                  <button
                    onClick={handleSend}
                    disabled={sending || thinking || !input.trim()}
                    className="w-9 h-9 rounded-full bg-primary flex items-center justify-center text-on-primary disabled:opacity-50 active:scale-95 transition-all"
                    aria-label="Send design request"
                  >
                    <Icon
                      name={sending || thinking ? "autorenew" : "send"}
                      size={18}
                      className={sending || thinking ? "animate-spin" : ""}
                    />
                  </button>
                </div>
              </div>
              <div className="flex justify-end mt-2 gap-2">
                <button
                  onClick={() =>
                    runPanel(async () => {
                      const r = await daemon.designVisualCritique(activeConvId!);
                      if (r.ok && r.visualCritique) {
                        setVisualCritique(r.visualCritique);
                        if (!r.visualCritique.available) {
                          setError(r.visualCritique.unavailable_reason || "Visual critic unavailable");
                        }
                      } else {
                        setError(r.error || "Could not run visual critique");
                      }
                    })
                  }
                  disabled={sending || thinking || panelBusy}
                  className="neo-button rounded-xl px-4 py-2 text-sm font-medium text-primary flex items-center gap-2 disabled:opacity-50"
                  aria-label="Run visual critic (gemma3:4b) on the live screenshot"
                  title="Independent visual critic (gemma3:4b) on the real rendered screenshot"
                >
                  <Icon name="visibility" size={16} />
                  Visual Critic
                </button>
                <button
                  onClick={() =>
                    runPanel(async () => {
                      const r = await daemon.designExecuteContract(activeConvId!);
                      if (r.ok && r.missionId) {
                        await refreshActive();
                        onOpenMission(r.missionId);
                      } else {
                        setError(r.error || "Could not create mission");
                      }
                    })
                  }
                  disabled={sending || thinking || panelBusy}
                  className="neo-button rounded-xl px-4 py-2 text-sm font-medium text-primary flex items-center gap-2 disabled:opacity-50"
                  aria-label="Implement via Mission with the full design contract"
                  title="Creates a mission carrying the full design contract: brief, constraints, QA findings, context"
                >
                  <Icon name="terminal" size={16} />
                  Implement via Mission
                </button>
              </div>
            </div>
          </>
        )}
      </div>

      {/* Right contextual design panel — also shown when the inbuilt
          browser is toggled open from the sidebar rail (watch-the-agent),
          even before any design conversation exists: the panel is a
          standalone surface that navigates any http(s) URL. */}
      {(activeConvId || panelOpen) && rightOpen && (
        <aside className="w-80 shrink-0 border-l border-outline-variant/40 dark:border-white/5 bg-surface/50 overflow-y-auto p-4 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-bold uppercase tracking-wider text-on-surface-variant">
              Design Workspace
            </h3>
            <div className="flex gap-1">
              <button
                onClick={handleUnderstand}
                disabled={panelBusy}
                className="neo-button rounded-lg px-2 py-1 text-[10px] font-medium text-on-surface-variant disabled:opacity-50"
                title="Understand product structure"
              >
                Understand
              </button>
              <button
                onClick={handlePreview}
                disabled={panelBusy}
                className="neo-button rounded-lg px-2 py-1 text-[10px] font-medium text-on-surface-variant disabled:opacity-50"
                title="Start the real preview server"
              >
                Preview
              </button>
              <button
                onClick={handleContract}
                disabled={panelBusy}
                className="neo-button rounded-lg px-2 py-1 text-[10px] font-medium text-on-surface-variant disabled:opacity-50"
                title="Build the executable design contract"
              >
                Contract
              </button>
              <button
                onClick={() => handleStandaloneQa("responsive")}
                disabled={panelBusy}
                className="neo-button rounded-lg px-2 py-1 text-[10px] font-medium text-on-surface-variant disabled:opacity-50"
                title="Responsive QA across real viewports"
              >
                QA·R
              </button>
              <button
                onClick={() => handleStandaloneQa("accessibility")}
                disabled={panelBusy}
                className="neo-button rounded-lg px-2 py-1 text-[10px] font-medium text-on-surface-variant disabled:opacity-50"
                title="Accessibility QA (contrast, targets, labels)"
              >
                QA·A11y
              </button>
              <button
                onClick={() => handleStandaloneQa("functional")}
                disabled={panelBusy}
                className="neo-button rounded-lg px-2 py-1 text-[10px] font-medium text-on-surface-variant disabled:opacity-50"
                title="Functional QA (interactions work)"
              >
                QA·F
              </button>
            </div>
          </div>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Product Understanding</h4>
              <button
                onClick={handleUnderstand}
                disabled={panelBusy}
                className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
              >
                Refresh
              </button>
            </div>
            {analysis ? (
              <div className="space-y-1.5 text-xs text-on-surface-variant">
                <p>
                  Framework:{" "}
                  <span className="text-on-surface font-medium">
                    {analysis.framework || "unknown"}
                  </span>
                </p>
                <p>
                  Routes: {analysis.routes.length} · Components: {analysis.components.length} ·
                  Styles: {analysis.style_files.length}
                </p>
                {analysis.components.slice(0, 6).map((c) => (
                  <p key={c} className="font-mono text-[10px] truncate">
                    {c}
                  </p>
                ))}
              </div>
            ) : (
              <p className="text-xs text-on-surface-variant">
                Scan the project for routes, components, styles, tokens and assets.
              </p>
            )}
          </section>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Reference Analysis</h4>
              <button
                onClick={handleAnalyzeReference}
                disabled={panelBusy}
                className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
                title="Analyze the latest attached reference image with a vision model"
              >
                Analyze
              </button>
            </div>
            {reference ? (
              <div className="space-y-1.5 text-xs text-on-surface-variant">
                <p>
                  Model:{" "}
                  <span className="font-mono text-[10px] text-on-surface">
                    {reference.vision_model}
                  </span>
                </p>
                {reference.adopted_principles.slice(0, 4).map((p, i) => (
                  <p key={i} className="text-[11px]">
                    · {p}
                  </p>
                ))}
                {reference.explicitly_do_not_copy.length > 0 && (
                  <p className="text-[10px] text-amber-600 dark:text-amber-400">
                    Do not copy: {reference.explicitly_do_not_copy.join("; ")}
                  </p>
                )}
              </div>
            ) : (
              <p className="text-xs text-on-surface-variant">
                Attach a reference image and extract structured design principles
                (vision model required).
              </p>
            )}
          </section>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Design Brief</h4>
              <button
                onClick={handleBrief}
                disabled={panelBusy}
                className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
              >
                Generate
              </button>
            </div>
            {brief ? (
              <div className="space-y-2">
                <p className="text-xs text-on-surface">
                  <span className="font-medium">{brief.product}</span> · {brief.audience}
                </p>
                <p className="text-xs text-on-surface-variant">
                  Workflow: {brief.primary_workflow} · Density: {brief.density}
                </p>
                {listTags(brief.visual_goals)}
                {brief.patterns_to_avoid.length > 0 && (
                  <div>
                    <p className="text-[10px] font-bold uppercase tracking-wide text-red-600 dark:text-red-400 mb-1">
                      Avoid
                    </p>
                    {listTags(brief.patterns_to_avoid)}
                  </div>
                )}
              </div>
            ) : (
              <p className="text-xs text-on-surface-variant">
                Create the structured product brief: audience, workflows, visual direction,
                hierarchy, accessibility and constraints.
              </p>
            )}
          </section>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Design Grammar</h4>
              <button
                onClick={handleGrammar}
                disabled={panelBusy}
                className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
              >
                Generate
              </button>
            </div>
            {grammar ? (
              <div className="space-y-2">
                {grammar.color_roles.length > 0 && (
                  <p className="text-xs text-on-surface-variant">
                    Colors: {grammar.color_roles.join("; ")}
                  </p>
                )}
                {listTags(grammar.type_scale)}
                {listTags(grammar.spacing)}
                {listTags(grammar.radii)}
                {listTags(grammar.component_principles)}
              </div>
            ) : (
              <p className="text-xs text-on-surface-variant">
                Establish typography, spacing, surfaces, color roles and component patterns
                specific to this product.
              </p>
            )}
          </section>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Design State</h4>
              <button
                onClick={handleDesignState}
                disabled={panelBusy}
                className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
              >
                Generate
              </button>
            </div>
            {designState ? (
              <pre className="text-[10px] text-on-surface-variant whitespace-pre-wrap font-mono max-h-40 overflow-y-auto">
                {designState.content}
              </pre>
            ) : (
              <p className="text-xs text-on-surface-variant">
                Persist durable project design decisions as DESIGN_STATE.md.
              </p>
            )}
          </section>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Browser / Screenshot</h4>
              <div className="flex items-center gap-1">
                <select
                  value={viewportHint}
                  onChange={(e) => setViewportHint(e.target.value)}
                  className="neo-input rounded-lg px-1.5 py-1 text-[10px] bg-transparent text-on-surface-variant"
                  aria-label="Viewport"
                >
                  <option value="compact">1024×768</option>
                  <option value="desktop">1440×900</option>
                  <option value="wide">1920×1080</option>
                </select>
                <button
                  onClick={handleBrowser}
                  disabled={panelBusy}
                  className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
                >
                  Inspect
                </button>
              </div>
            </div>
            {browser ? (
              <div className="space-y-2">
                <p className="text-xs text-on-surface-variant font-mono truncate">{browser.url}</p>
                <p className="text-[10px] text-on-surface-variant">
                  HTTP {browser.diagnostics.http_status} · console errors{" "}
                  {browser.diagnostics.console_errors.length} · network failures{" "}
                  {browser.diagnostics.network_failures.length}
                </p>
                <p className="text-[10px] font-mono text-on-surface-variant truncate">
                  {browser.screenshot_uri}
                </p>
                {qa.responsive && (
                  <p className="text-[10px] flex items-center gap-1">
                    <Icon
                      name={qa.responsive.passed ? "check_circle" : "cancel"}
                      size={12}
                      className={qa.responsive.passed ? "text-emerald-600" : "text-red-600"}
                    />
                    Responsive: {qa.responsive.passed ? "pass" : `${qa.responsive.issues.length} issue(s)`}
                    {Array.isArray(qa.responsive.unmeasured) && qa.responsive.unmeasured.length > 0 && (
                      <span className="text-amber-600 dark:text-amber-400">
                        {" "}· {qa.responsive.unmeasured.length} unmeasured (manual review)
                      </span>
                    )}
                  </p>
                )}
                {qa.accessibility && (
                  <p className="text-[10px] flex items-center gap-1">
                    <Icon
                      name={qa.accessibility.passed ? "check_circle" : "cancel"}
                      size={12}
                      className={qa.accessibility.passed ? "text-emerald-600" : "text-red-600"}
                    />
                    Accessibility: {qa.accessibility.passed ? "pass" : `${qa.accessibility.issues.length} issue(s)`}
                    {Array.isArray(qa.accessibility.unmeasured) && qa.accessibility.unmeasured.length > 0 && (
                      <span className="text-amber-600 dark:text-amber-400">
                        {" "}· {qa.accessibility.unmeasured.length} unmeasured (manual review)
                      </span>
                    )}
                  </p>
                )}
                {qa.functional && (
                  <p className="text-[10px] flex items-center gap-1">
                    <Icon
                      name={qa.functional.passed ? "check_circle" : "cancel"}
                      size={12}
                      className={qa.functional.passed ? "text-emerald-600" : "text-red-600"}
                    />
                    Functional: {qa.functional.passed ? "pass" : `${qa.functional.issues.length} issue(s)`}
                  </p>
                )}
              </div>
            ) : (
              <p className="text-xs text-on-surface-variant">
                Inspect the running application: navigate, read the DOM, capture console and
                network diagnostics, and produce screenshot evidence.
              </p>
            )}
          </section>

          {browserPanelSection}

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Visual Critic</h4>
              <span className="text-[10px] text-on-surface-variant">
                gemma3:4b · independent of anti-slop
              </span>
            </div>
            {visualCritique ? (
              visualCritique.available ? (
                <div className="space-y-1.5">
                  <p className="text-xs text-on-surface">{visualCritique.overall}</p>
                  {visualCritique.findings.map((f, i) => (
                    <div key={i} className="text-[11px] text-on-surface-variant">
                      <span className="text-red-600 dark:text-red-400 font-medium">
                        [{f.aspect ?? "?"}] {f.issue}
                      </span>
                      {f.suggestion && <span className="block pl-2">→ {f.suggestion}</span>}
                    </div>
                  ))}
                  {(visualCritique.strengths ?? []).length > 0 && (
                    <p className="text-[10px] text-emerald-600 dark:text-emerald-400">
                      Strengths: {visualCritique.strengths?.join("; ")}
                    </p>
                  )}
                </div>
              ) : (
                <p className="text-xs text-amber-600 dark:text-amber-400">
                  {visualCritique.unavailable_reason}
                </p>
              )
            ) : (
              <p className="text-xs text-on-surface-variant">
                An independent vision model critiques the real screenshot — a separate
                signal from the deterministic anti-slop critique.
              </p>
            )}
          </section>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Generate Mockups</h4>
              <span className="text-[10px] text-on-surface-variant">prompt → variants → scored winner</span>
            </div>
            <textarea
              value={mockupPrompt}
              onChange={(e) => setMockupPrompt(e.target.value)}
              placeholder="Describe the screen (e.g. “landing page for the mission terminal”)"
              rows={2}
              className="w-full neo-pressed rounded-lg p-2 text-xs text-on-surface bg-transparent resize-none focus:outline-none"
            />
            <button
              onClick={handleGenerateMockups}
              disabled={panelBusy}
              className="mt-2 neo-button rounded-lg px-3 py-1.5 text-[11px] font-medium text-on-surface-variant disabled:opacity-50"
              title="Generate candidate mockups, rendered and scored in the real browser"
            >
              Generate variants
            </button>
            {mockups && (
              <div className="mt-3 space-y-2">
                {mockups.spec_parse_failed && (
                  <p className="text-[10px] text-amber-600">
                    The model's spec could not be parsed — a minimal spec was used honestly.
                  </p>
                )}
                {mockups.variants.map((v) => (
                  <div key={v.variant} className="neo-pressed rounded-lg p-2">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-[10px] font-medium text-on-surface">
                        Variant {v.variant + 1} · {v.layout} · {v.palette_intent}
                      </span>
                      {v.variant === mockups.winner && (
                        <span className="text-[10px] font-medium text-emerald-600">★ WINNER</span>
                      )}
                      <span className="text-[10px] text-on-surface-variant">
                        {v.score?.source === "real-browser"
                          ? `browser ✓ ${v.score?.visible_chars ?? 0} chars`
                          : v.score?.source === "structural"
                            ? "structural"
                            : "unavailable"}
                      </span>
                    </div>
                    <iframe
                      title={`Mockup variant ${v.variant + 1}`}
                      srcDoc={v.html}
                      sandbox=""
                      className="w-full h-40 rounded border-0 bg-white"
                    />
                  </div>
                ))}
                <div className="mt-2 flex items-center gap-2">
                  <input
                    value={exportTarget}
                    onChange={(e) => setExportTarget(e.target.value)}
                    placeholder="src/pages/GeneratedLanding.tsx"
                    className="flex-1 neo-pressed rounded-lg px-2 py-1 text-[11px] text-on-surface bg-transparent focus:outline-none"
                  />
                  <button
                    onClick={handleExportWinner}
                    disabled={panelBusy}
                    className="neo-button rounded-lg px-3 py-1.5 text-[11px] font-medium text-on-surface-variant disabled:opacity-50"
                    title="Promote the winning mockup to a governed implementation mission"
                  >
                    Export winner → mission
                  </button>
                </div>
                {exportResult && (
                  <p className="mt-2 text-[10px] text-emerald-600">
                    Mission {exportResult.missionId.slice(0, 18)}… implementing {exportResult.target} — watch it in Missions.
                  </p>
                )}
              </div>
            )}
          </section>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Multi-Screen Flow</h4>
              <span className="text-[10px] text-on-surface-variant">
                one prompt per line → one scored screen each
              </span>
            </div>
            <textarea
              value={flowScreens}
              onChange={(e) => setFlowScreens(e.target.value)}
              placeholder={"Landing — hero + primary CTA\nPricing — three tiers\nDocs — search + nav"}
              rows={3}
              className="w-full neo-pressed rounded-lg p-2 text-xs text-on-surface bg-transparent resize-none focus:outline-none"
            />
            <button
              onClick={handleGenerateFlow}
              disabled={panelBusy}
              className="mt-2 neo-button rounded-lg px-3 py-1.5 text-[11px] font-medium text-on-surface-variant disabled:opacity-50"
              title="Generate one scored mockup run per screen and group the winners into a flow"
            >
              Generate flow
            </button>
            {flow && (
              <div className="mt-3 space-y-2">
                <p className="text-[10px] text-on-surface-variant">
                  Flow {flow.flow_id?.slice(0, 18)}… · {flow.screen_count} screen(s)
                </p>
                {(flow.failed_screens ?? []).length > 0 && (
                  <p className="text-[10px] text-amber-600">
                    {flow.failed_screens?.length} screen(s) failed honestly — see their errors below.
                  </p>
                )}
                {(flow.screens ?? []).map((s) => (
                  <div key={s.index} className="neo-pressed rounded-lg p-2">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-[10px] font-medium text-on-surface">
                        Screen {s.index + 1} · {s.screen.slice(0, 50)}
                      </span>
                      <span className="text-[10px] text-emerald-600">★ winner #{(s.winner ?? 0) + 1}</span>
                    </div>
                    {s.winner_html && (
                      <iframe
                        title={`Flow screen ${s.index + 1} winner`}
                        srcDoc={s.winner_html}
                        sandbox=""
                        className="w-full h-32 rounded border-0 bg-white"
                      />
                    )}
                  </div>
                ))}
                {(flow.failed_screens ?? []).map((f) => (
                  <p key={`failed-${f.index}`} className="text-[10px] text-red-600">
                    Screen {f.index + 1} ({f.screen.slice(0, 30)}): {f.error}
                  </p>
                ))}
              </div>
            )}
          </section>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Design Contract</h4>
            </div>
            {contract ? (
              <pre className="text-[10px] font-mono text-on-surface-variant overflow-auto max-h-48 neo-pressed rounded-lg p-2">
                {JSON.stringify(contract, null, 2)}
              </pre>
            ) : (
              <p className="text-xs text-on-surface-variant">Run “Contract” to build the executable specification.</p>
            )}
          </section>

          {standaloneQa && (
            <section className="neo-raised rounded-2xl p-4">
              <div className="flex items-center justify-between mb-2">
                <h4 className="text-sm font-semibold text-on-surface">QA Report</h4>
                <span className={`text-[10px] font-medium ${standaloneQa.passed ? "text-emerald-600" : "text-amber-600"}`}>
                  {standaloneQa.passed ? "PASSED" : "ISSUES FOUND"}
                </span>
              </div>
              <ul className="space-y-1 text-xs text-on-surface-variant">
                {(standaloneQa.issues ?? []).map((issue, i) => (
                  <li key={i} className="flex items-start gap-2">
                    <span className="text-amber-600">!</span>
                    <span className="flex-1">{issue}</span>
                  </li>
                ))}
                {(standaloneQa.issues ?? []).length === 0 && (
                  <li className="text-emerald-600">All checks passed.</li>
                )}
              </ul>
            </section>
          )}

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">
                Durable Constraints
              </h4>
              <span className="text-[10px] text-on-surface-variant">
                project-wide · outrank aesthetics
              </span>
            </div>
            <ul className="space-y-1 mb-2">
              {constraints.map((c, i) => (
                <li key={i} className="text-[11px] text-on-surface flex items-start gap-1">
                  <button
                    onClick={() =>
                      runPanel(async () => {
                        const next = constraints.filter((_, index) => index !== i);
                        const r = await daemon.designConstraintsSet(activeConvId!, next);
                        if (r.ok && r.constraints) setConstraints(r.constraints.constraints ?? []);
                      })
                    }
                    className="text-red-600 dark:text-red-400 hover:opacity-70 shrink-0"
                    aria-label={`Remove constraint: ${c.text}`}
                  >
                    <Icon name="close" size={12} />
                  </button>
                  <span>{c.text}</span>
                </li>
              ))}
            </ul>
            {constraints.length === 0 && (
              <p className="text-[10px] text-on-surface-variant mb-2">
                No constraints set. Constraints persist across every design chat in this
                project and are marked non-negotiable in design contracts.
              </p>
            )}
            <div className="flex gap-1.5">
              <input
                className="neo-input rounded-xl px-3 py-1.5 text-xs flex-1"
                placeholder="e.g. Data density outranks whitespace"
                value={constraintDraft}
                onChange={(e) => setConstraintDraft(e.target.value)}
                onKeyDown={(e: KeyboardEvent) => {
                  if (e.key === "Enter" && constraintDraft.trim() && activeConvId) {
                    e.preventDefault();
                    runPanel(async () => {
                      const next = [
                        ...constraints,
                        { text: constraintDraft.trim() },
                      ];
                      const r = await daemon.designConstraintsSet(activeConvId!, next);
                      if (r.ok && r.constraints) {
                        setConstraints(r.constraints.constraints ?? []);
                        setConstraintDraft("");
                      } else {
                        setError(r.error || "Could not save constraint");
                      }
                    });
                  }
                }}
              />
              <button
                onClick={() =>
                  runPanel(async () => {
                    if (!constraintDraft.trim() || !activeConvId) return;
                    const next = [...constraints, { text: constraintDraft.trim() }];
                    const r = await daemon.designConstraintsSet(activeConvId!, next);
                    if (r.ok && r.constraints) {
                      setConstraints(r.constraints.constraints ?? []);
                      setConstraintDraft("");
                    } else {
                      setError(r.error || "Could not save constraint");
                    }
                  })
                }
                disabled={!constraintDraft.trim() || panelBusy}
                className="rounded-xl px-3 py-1.5 text-xs font-medium bg-primary text-on-primary disabled:opacity-50"
                aria-label="Add constraint"
              >
                Add
              </button>
            </div>
          </section>

          <section className="neo-raised rounded-2xl p-4">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-sm font-semibold text-on-surface">Critique &amp; Repair</h4>
              <div className="flex gap-1">
                <button
                  onClick={handleCritique}
                  disabled={panelBusy}
                  className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
                >
                  Critique
                </button>
                <button
                  onClick={handleRepair}
                  disabled={panelBusy}
                  className="text-[10px] text-amber-600 dark:text-amber-400 hover:opacity-80 disabled:opacity-50"
                >
                  Repair
                </button>
                <button
                  onClick={handleDesignMemory}
                  disabled={panelBusy}
                  className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
                >
                  Memory
                </button>
              </div>
            </div>
            {critique && (
              <div className="space-y-2">
                <p className="text-xs flex items-center gap-1.5">
                  <Icon
                    name={critique.passed ? "verified_user" : "report_problem"}
                    size={14}
                    className={critique.passed ? "text-emerald-600" : "text-red-600"}
                  />
                  <span className={critique.passed ? "text-emerald-600" : "text-red-600"}>
                    {critique.passed ? "Critique passed" : "Issues found"}
                  </span>
                </p>
                {critique.findings.map((f, i) => (
                  <div key={i} className="space-y-0.5">
                    <p className={`text-[10px] ${severityColor(f.severity)}`}>
                      {f.rule} — {f.explanation}
                    </p>
                    {f.constraint_violations && f.constraint_violations.length > 0 && (
                      <p className="text-[10px] text-red-600 dark:text-red-400 pl-2">
                        constraint check: {f.constraint_violations.join("; ")} — repair must respect
                        durable constraints
                      </p>
                    )}
                  </div>
                ))}
              </div>
            )}
            {repair && repair.repairs.length > 0 && (
              <div className="mt-3 space-y-2 border-t border-outline-variant/40 pt-2">
                {repair.iteration != null && (
                  <p className="text-[10px] text-primary font-medium">
                    Iteration {repair.iteration}
                    {repair.remaining_issues && repair.remaining_issues.length > 0
                      ? ` — ${repair.remaining_issues.length} remaining`
                      : " — no remaining issues"}
                  </p>
                )}
                {repair.repairs.map((r, i) => (
                  <p key={i} className="text-[10px] text-on-surface-variant">
                    <span className="text-amber-600 dark:text-amber-400">{r.issue}:</span> {r.repair}
                  </p>
                ))}
                <button
                  onClick={handleIterationHistory}
                  disabled={panelBusy}
                  className="text-[10px] text-primary hover:opacity-80 disabled:opacity-50"
                >
                  Iteration History
                </button>
              </div>
            )}
            {iterationHistory && iterationHistory.length > 0 && (
              <div className="mt-2 space-y-1 border-t border-outline-variant/40 pt-2">
                <p className="text-[10px] font-semibold text-on-surface">Repair-loop history</p>
                {iterationHistory.map((it, i) => (
                  <p key={i} className="text-[10px] text-on-surface-variant">
                    #{it.iteration} {it.passed ? "✓" : "•"}
                    {it.remaining_issues && it.remaining_issues.length > 0
                      ? ` — ${it.remaining_issues.join(", ")}`
                      : " — clean"}
                  </p>
                ))}
              </div>
            )}
            {designMemory && (
              <div className="mt-2 space-y-1 border-t border-outline-variant/40 pt-2">
                <p className="text-[10px] font-semibold text-on-surface">Design memory (project)</p>
                {designMemory.inherited_from_conversation && (
                  <p className="text-[10px] text-on-surface-variant">
                    inherited from a previous design chat
                  </p>
                )}
                {designMemory.constraints.length > 0 && (
                  <p className="text-[10px] text-on-surface-variant">
                    {designMemory.constraints.length} durable constraint(s)
                  </p>
                )}
                {designMemory.accepted_decisions.length > 0 && (
                  <p className="text-[10px] text-on-surface-variant">
                    {designMemory.accepted_decisions.length} accepted decision(s)
                  </p>
                )}
                <p className="text-[10px] text-on-surface-variant">
                  brief/grammar/reference: {designMemory.brief ? "yes" : "no"} /{" "}
                  {designMemory.grammar ? "yes" : "no"} /{" "}
                  {designMemory.reference_principles ? "yes" : "no"}
                </p>
              </div>
            )}
            {!critique && !repair && (
              <p className="text-xs text-on-surface-variant">
                Critique the rendered application for generic-AI patterns, hierarchy, spacing,
                accessibility, and functional preservation — then repair material issues.
              </p>
            )}
          </section>

          {panelBusy && (
            <div className="flex items-center gap-2 text-xs text-on-surface-variant">
              <Icon name="autorenew" size={14} className="animate-spin" />
              Working…
            </div>
          )}
        </aside>
      )}
    </main>
  );
}