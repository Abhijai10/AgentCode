import { useState, useEffect, useRef } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { Project } from "./ProjectContext";
import type { ProjectMemory } from "./types";

export function HomeView({
  project,
  onOpenProject,
  onSubmit,
  onConfigureProviders,
  onNavigate,
}: {
  project: Project | null;
  onOpenProject(): void;
  onSubmit(goal: string): Promise<{ ok: boolean; error?: string }>;
  onConfigureProviders(): void;
  onNavigate(view: "design" | "mission" | "settings"): void;
}) {
  const [goal, setGoal] = useState("");
  const [hasUsableRoute, setHasUsableRoute] = useState(true);
  const [routeCheckDone, setRouteCheckDone] = useState(false);
  // Batch N7: real first-run readiness from the daemon (dismissible).
  const [readiness, setReadiness] = useState<{
    daemon_lifecycle: string;
    provider_accounts_configured: number;
    ollama: { running: boolean; model_count: number };
    scanner_note: string;
    usable_route: boolean;
  } | null>(null);
  const [readinessDismissed, setReadinessDismissed] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  // F1: durable project memory (facts/decisions/task memories persisted by
  // previous missions — shared by every mode).
  const [memory, setMemory] = useState<ProjectMemory | null>(null);
  useEffect(() => {
    if (!project) {
      setMemory(null);
      return;
    }
    let cancelled = false;
    (async () => {
      const result = await daemon.projectMemoryGet(project.path);
      if (!cancelled) setMemory(result);
    })();
    return () => {
      cancelled = true;
    };
  }, [project]);
  // Batch N7: the whole composer surface reads as the input — clicking
  // anywhere focuses the editor instead of only the textarea itself.
  const composerRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!project) return;
    let cancelled = false;
    (async () => {
      const [providers, o, ready] = await Promise.all([
        daemon.listProviders(),
        daemon.discoverOllama(),
        daemon.readinessGet(),
      ]);
      if (cancelled) return;
      const hasAny = providers.some((p) => p.connected_accounts > 0 && p.health === "healthy");
      setHasUsableRoute(hasAny || o.running);
      setRouteCheckDone(true);
      if (ready.ok && ready.readiness) setReadiness(ready.readiness);
    })();
    return () => {
      cancelled = true;
    };
  }, [project]);

  if (!project) {
    return (
      <main className="flex-1 overflow-y-auto p-8 relative flex items-center justify-center">
        <div className="max-w-2xl mx-auto flex flex-col gap-8 w-full">
          <div className="text-center space-y-4">
            <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl neo-raised mb-4 overflow-hidden">
              <img src="/logo.png" className="w-full h-full object-cover" alt="AgentCode" />
            </div>
            <h2 className="text-4xl font-semibold text-on-surface tracking-tight">Welcome to AgentCode</h2>
            <p className="text-on-surface-variant text-lg max-w-xl mx-auto">Open an existing project or start a new one from scratch to begin building.</p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <button
              onClick={() => {
                onOpenProject();
              }}
              className="neo-raised rounded-3xl p-8 flex flex-col gap-5 text-left hover:text-primary transition-all duration-200 active:scale-[0.98]"
            >
              <div className="w-14 h-14 rounded-full neo-pressed flex items-center justify-center text-primary">
                <Icon name="create_new_folder" size={28} />
              </div>
              <div>
                <h3 className="font-semibold text-on-surface text-lg mb-1">Start a New Project</h3>
                <p className="text-sm text-on-surface-variant leading-relaxed">Create a fresh project. Choose where it lives on your machine and give it a name.</p>
              </div>
            </button>
            <button
              onClick={() => {
                onOpenProject();
              }}
              className="neo-raised rounded-3xl p-8 flex flex-col gap-5 text-left hover:text-primary transition-all duration-200 active:scale-[0.98]"
            >
              <div className="w-14 h-14 rounded-full neo-pressed flex items-center justify-center text-primary">
                <Icon name="folder_open" size={28} />
              </div>
              <div>
                <h3 className="font-semibold text-on-surface text-lg mb-1">Open a Project</h3>
                <p className="text-sm text-on-surface-variant leading-relaxed">Browse to an existing folder on your computer and start working on it.</p>
              </div>
            </button>
          </div>

          <div className="flex items-center justify-center gap-2 text-xs text-on-surface-variant mt-2">
            <Icon name="info" size={14} />
            Both options open a folder picker to select the project location.
          </div>
        </div>
      </main>
    );
  }

  const send = async () => {
    const trimmed = goal.trim();
    if (!trimmed || submitting) return;
    setSubmitting(true);
    setSubmitError(null);
    const result = await onSubmit(trimmed);
    if (result.ok) {
      setGoal("");
    } else {
      // Surface the real backend error (e.g. a rejected workspace_root or a
      // provider/down-daemon failure) instead of a generic placeholder.
      setSubmitError(result.error || "Mission submission failed.");
    }
    setSubmitting(false);
  };

  return (
    <main className="flex-1 overflow-y-auto p-8 relative">
      <div className="max-w-3xl mx-auto flex flex-col gap-8 h-full justify-center pb-20">
        <div className="text-center space-y-4 mb-8">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl neo-raised mb-4 overflow-hidden">
            <img src="/logo.png" className="w-full h-full object-cover" alt="AgentCode" />
          </div>
          <h2 className="text-4xl font-semibold text-on-surface tracking-tight">What do you want to build?</h2>
          <p className="text-on-surface-variant text-lg max-w-xl mx-auto">Working in <span className="font-semibold text-primary">{project.name}</span>. Describe your vision — the agent will orchestrate design, logic, and infrastructure to bring it to life.</p>
          <p className="text-xs text-on-surface-variant font-mono max-w-xl mx-auto truncate">{project.path}</p>
        </div>

        {memory && (memory.counts.facts > 0 || memory.counts.task_memories > 0) && (
          <details className="neo-raised rounded-2xl px-4 py-3 group" open>
            <summary className="cursor-pointer text-sm font-medium text-on-surface flex items-center gap-2 list-none">
              <Icon name="brain" size={16} />
              Project memory
              <span className="text-xs text-on-surface-variant">
                {memory.counts.facts} facts · {memory.counts.task_memories} task notes
                {memory.counts.decisions > 0 ? ` · ${memory.counts.decisions} decisions` : ""}
              </span>
            </summary>
            <div className="mt-3 space-y-3 text-left">
              {memory.facts.length > 0 && (
                <div>
                  <p className="text-xs font-medium text-on-surface-variant mb-1">Learned facts</p>
                  <ul className="space-y-1">
                    {memory.facts.slice(0, 5).map((f) => (
                      <li key={f.id} className="text-xs text-on-surface flex items-start gap-2">
                        <span className="text-primary mt-0.5">•</span>
                        <span className="flex-1">{f.statement}</span>
                        <span className="opacity-60 font-mono text-[10px]">{f.confidence}%</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              {memory.task_memories.length > 0 && (
                <div>
                  <p className="text-xs font-medium text-on-surface-variant mb-1">Recent task outcomes</p>
                  <ul className="space-y-1">
                    {memory.task_memories.slice(0, 3).map((t) => (
                      <li key={t.id} className="text-xs text-on-surface flex items-start gap-2">
                        <span className="text-primary mt-0.5">•</span>
                        <span className="flex-1">{t.summary}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </details>
        )}

        {readiness && !readinessDismissed && !readiness.usable_route && (
          <div className="neo-raised rounded-2xl p-4 mb-4 flex items-start justify-between gap-3">
            <div className="text-left">
              <p className="text-sm font-medium text-on-surface">First-run readiness</p>
              <ul className="text-xs text-on-surface-variant mt-1 space-y-0.5">
                <li>
                  daemon:{" "}
                  <span className="text-emerald-600">{readiness.daemon_lifecycle}</span>
                </li>
                <li>
                  provider accounts:{" "}
                  <span className={readiness.provider_accounts_configured > 0 ? "text-emerald-600" : "text-amber-600"}>
                    {readiness.provider_accounts_configured} configured
                  </span>
                </li>
                <li>
                  local models (Ollama):{" "}
                  <span className={readiness.ollama.running ? "text-emerald-600" : "text-amber-600"}>
                    {readiness.ollama.running ? `${readiness.ollama.model_count} available` : "not running"}
                  </span>
                </li>
                <li className="opacity-70">{readiness.scanner_note}</li>
              </ul>
            </div>
            <button
              onClick={() => setReadinessDismissed(true)}
              className="text-on-surface-variant hover:opacity-80 text-xs shrink-0"
            >
              Dismiss
            </button>
          </div>
        )}
        {routeCheckDone && !hasUsableRoute && (
          <div className="neo-raised p-6 rounded-2xl text-center">
            <div className="w-12 h-12 rounded-full neo-pressed mx-auto mb-3 flex items-center justify-center text-primary">
              <Icon name="route" size={24} />
            </div>
            <h4 className="font-semibold text-on-surface">No model routes configured</h4>
            <p className="text-sm text-on-surface-variant mt-2 max-w-lg mx-auto">
              AgentCode needs at least one usable model route. Connect a provider account or start a local model to begin.
            </p>
            <div className="flex justify-center gap-3 mt-5">
              <button
                onClick={onConfigureProviders}
                className="px-6 py-3 rounded-xl bg-primary text-on-primary text-sm font-medium hover:brightness-110 transition-all"
              >
                Configure Provider
              </button>
              <button
                onClick={onConfigureProviders}
                className="px-6 py-3 rounded-xl neo-button text-sm text-primary font-medium"
              >
                Use Local Model
              </button>
            </div>
          </div>
        )}

        {submitError && (
  <div className="flex items-center gap-2 text-sm text-red-600 dark:text-red-400 font-medium px-2">
    <Icon name="error" size={16} fill /> {submitError}
  </div>
)}
<div
          className="neo-raised rounded-[24px] p-2 flex flex-col relative cursor-text"
          onClick={(e) => {
            // Click anywhere in the intended input area focuses the editor.
            // Don't steal focus from interactive children (buttons/links).
            if (!(e.target instanceof HTMLElement && e.target.closest("button, a, input, select"))) {
              composerRef.current?.focus();
            }
          }}
        >
          <div className="neo-pressed rounded-[20px] p-6 min-h-[160px] flex flex-col">
<textarea
                ref={composerRef}
                className="w-full bg-transparent border-none outline-none resize-none text-on-surface placeholder:text-on-surface-variant/50 text-lg flex-1 focus:ring-0 p-0 disabled:opacity-50"
                placeholder={`e.g., Build a real-time dashboard in ${project.name} for monitoring satellite telemetry data...`}
                value={goal}
                disabled={submitting}
                onChange={(e) => setGoal(e.target.value)}
                onKeyDown={async (e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    await send();
                  }
                }}
              />
            <div className="flex items-center mt-4 pt-4 border-t border-outline-variant/40 dark:border-white/5">
              <button
                onClick={() => send()}
                disabled={submitting || !goal.trim()}
                className="ml-auto w-12 h-12 rounded-full bg-surface shadow-neo-raised-primary flex items-center justify-center text-primary hover:text-primary-container transition-all duration-150 active:shadow-neo-pressed disabled:opacity-50"
              >
                <Icon name={submitting ? "autorenew" : "send"} size={24} fill className={submitting ? "animate-spin" : ""} />
              </button>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mt-4">
          <button
            onClick={() => onNavigate("design")}
            className="neo-raised p-5 rounded-2xl flex flex-col gap-3 text-left hover:text-primary transition-colors duration-200 active:scale-[0.98]"
          >
            <div className="w-10 h-10 rounded-full neo-pressed flex items-center justify-center text-primary">
              <Icon name="design_services" size={20} />
            </div>
            <div>
              <h3 className="font-semibold text-on-surface text-sm mb-1">Current Plan</h3>
              <p className="text-xs text-on-surface-variant">View task plan and dependencies</p>
            </div>
          </button>
          <button
            onClick={() => onNavigate("mission")}
            className="neo-raised p-5 rounded-2xl flex flex-col gap-3 text-left hover:text-primary transition-colors duration-200 active:scale-[0.98]"
          >
            <div className="w-10 h-10 rounded-full neo-pressed flex items-center justify-center text-tertiary">
              <Icon name="terminal" size={20} />
            </div>
            <div>
              <h3 className="font-semibold text-on-surface text-sm mb-1">Active Mission</h3>
              <p className="text-xs text-on-surface-variant">View mission progress and activity</p>
            </div>
          </button>
          <button
            onClick={() => onNavigate("settings")}
            className="neo-raised p-5 rounded-2xl flex flex-col gap-3 text-left hover:text-primary transition-colors duration-200 active:scale-[0.98]"
          >
            <div className="w-10 h-10 rounded-full neo-pressed flex items-center justify-center text-primary">
              <Icon name="tune" size={20} />
            </div>
            <div>
              <h3 className="font-semibold text-on-surface text-sm mb-1">Configuration</h3>
              <p className="text-xs text-on-surface-variant">Manage providers and settings</p>
            </div>
          </button>
        </div>
      </div>
    </main>
  );
}