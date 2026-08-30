import { useEffect, useState } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import { useProject } from "./ProjectContext";
import type { DesignSession } from "./types";

export function DesignView() {
  const { project } = useProject();
  const [prompt, setPrompt] = useState("");
  const [sessions, setSessions] = useState<DesignSession[]>([]);
  const [activeSession, setActiveSession] = useState<DesignSession | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const refresh = async () => {
      const s = await daemon.designListSessions();
      if (!cancelled) {
        setSessions(s);
        if (!activeSession && s.length > 0) {
          const detail = await daemon.designGetSession(s[0].id);
          if (!cancelled && detail) setActiveSession(detail);
        }
      }
    };
    refresh();
    const timer = setInterval(refresh, 4000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [activeSession]);

  const selectSession = async (id: string) => {
    const detail = await daemon.designGetSession(id);
    if (detail) setActiveSession(detail);
  };

  const handleDesign = async () => {
    const trimmed = prompt.trim();
    if (!trimmed || busy) return;
    setPrompt("");
    setBusy(true);
    setError(null);
    // The design backend contract is not yet available through the daemon IPC.
    // Design sessions are owned by the backend; submitting a mission is the
    // real daemon path for goal-driven work.
    const mission = await daemon.submitMission(trimmed, project?.path);
    if (mission.ok) {
      const s = await daemon.designListSessions();
      setSessions(s);
    } else {
      setError(mission.error || "The daemon is not available. Design sessions require the backend design mode.");
    }
    setBusy(false);
  };

  return (
    <main className="flex-1 flex flex-col overflow-hidden">
      <div className="p-6 flex flex-col gap-6 overflow-y-auto flex-1">
        <div className="neo-raised rounded-2xl p-2 flex items-center gap-3 w-full max-w-4xl mx-auto">
          <div className="flex-1 relative">
            <div className="absolute inset-y-0 left-4 flex items-center pointer-events-none text-outline">
              <Icon name="magic_button" size={20} />
            </div>
            <input
              className="w-full bg-transparent border-none py-4 pl-12 pr-4 text-on-surface focus:ring-0 neo-pressed rounded-xl"
              placeholder="Tell AgentCode what you want..."
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleDesign();
                }
              }}
            />
          </div>
          <div className="flex gap-2 pr-2">
            <button
              onClick={handleDesign}
              className="px-6 py-3 rounded-xl neo-raised text-primary font-bold text-sm flex items-center gap-2"
            >
              <Icon name="auto_fix_high" size={16} fill />
              {busy ? "Submitting..." : "Design"}
            </button>
          </div>
        </div>

        {error && (
          <div className="w-full max-w-4xl mx-auto flex items-center gap-2 text-sm text-red-600 dark:text-red-400">
            <Icon name="error" size={16} fill />
            {error}
          </div>
        )}

        <div className="flex-1 w-full max-w-6xl mx-auto flex flex-col md:flex-row gap-6 pb-8 min-h-0">
          <div className="w-full md:w-64 flex flex-col gap-4">
            <div className="neo-raised rounded-2xl p-5 flex-1 overflow-y-auto">
              <h3 className="font-semibold text-on-surface mb-4 flex items-center gap-2">
                <Icon name="layers" size={16} className="text-primary" />
                Sessions
              </h3>
              {sessions.length === 0 && (
                <p className="text-xs text-on-surface-variant">
                  No design sessions yet. Design sessions are owned by the backend; they appear here once recorded.
                </p>
              )}
              <div className="space-y-2">
                {sessions.map((m) => (
                  <button
                    key={m.id}
                    onClick={() => selectSession(m.id)}
                    className={`text-left text-sm p-3 rounded-xl w-full ${activeSession?.id === m.id ? "neo-pressed" : "neo-button"}`}
                  >
                    <p className="truncate text-on-surface font-medium">{m.product}</p>
                    <p className="text-[11px] text-on-surface-variant mt-1">{m.state} · {m.artifacts.length} artifact{m.artifacts.length !== 1 ? "s" : ""}</p>
                  </button>
                ))}
              </div>
            </div>
          </div>

          <div className="flex-1 neo-pressed rounded-3xl p-4 flex flex-col overflow-hidden min-h-0">
            <div className="flex items-center gap-2 mb-4 px-2">
              <div className="flex gap-1.5">
                <div className="w-3 h-3 rounded-full bg-outline-variant" />
                <div className="w-3 h-3 rounded-full bg-outline-variant" />
                <div className="w-3 h-3 rounded-full bg-outline-variant" />
              </div>
              <div className="flex-1 flex justify-center">
                <div className="neo-raised rounded-full px-4 py-1.5 text-xs text-on-surface-variant flex items-center gap-2 w-72 max-w-full justify-center">
                  <Icon name="lock" size={14} />
                  Design Studio — backend state
                </div>
              </div>
            </div>
            <div className="flex-1 bg-surface-container-lowest rounded-2xl border border-outline-variant/20 overflow-hidden relative flex items-center justify-center">
              {activeSession ? (
                <div className="p-8 w-full h-full overflow-y-auto">
                  <h2 className="font-semibold text-lg text-on-surface mb-4">{activeSession.product}</h2>
                  <div className="flex items-center gap-2 mb-6">
                    <span className={`text-[11px] px-2 py-0.5 rounded-full font-medium ${
                      activeSession.state === "completed" ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" :
                      activeSession.state === "running" || activeSession.state === "active" ? "bg-primary/10 text-primary" :
                      "bg-on-surface-variant/10 text-on-surface-variant"
                    }`}>{activeSession.state}</span>
                  </div>
                  {activeSession.artifacts.length === 0 ? (
                    <p className="text-sm text-on-surface-variant">No artifacts recorded for this session yet.</p>
                  ) : (
                    <div className="space-y-4">
                      {activeSession.artifacts.map((a) => (
                        <div key={a.id} className="neo-raised rounded-xl p-4">
                          <div className="flex items-center justify-between">
                            <h4 className="font-semibold text-on-surface text-sm">{a.name}</h4>
                            <span className="text-[11px] px-2 py-0.5 rounded-full bg-primary/10 text-primary font-medium">{a.artifact_type}</span>
                          </div>
                          <p className="text-xs text-on-surface-variant mt-1">Version {a.current_version} · {a.versions.length} version{a.versions.length !== 1 ? "s" : ""}</p>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              ) : (
                <div className="text-center p-8">
                  <div className="w-24 h-24 rounded-2xl neo-raised mx-auto mb-6 flex items-center justify-center">
                    <Icon name="design_services" size={40} className="text-primary animate-pulse" />
                  </div>
                  <h2 className="font-semibold text-2xl text-on-surface mb-2">Design Studio</h2>
                  <p className="text-on-surface-variant max-w-md mx-auto mb-8 text-sm">
                    Enter a prompt above to request a design. Design artifacts are owned and recorded by the backend.
                  </p>
                </div>
              )}
            </div>
          </div>

          <div className="w-full md:w-72 flex flex-col gap-4">
            <div className="neo-raised rounded-2xl p-5 flex-1">
              <h3 className="font-semibold text-on-surface mb-6 flex items-center gap-2">
                <Icon name="tune" size={16} className="text-primary" />
                Details
              </h3>
              <div className="space-y-3 text-xs text-on-surface-variant">
                <p>Design sessions, artifacts and versions are stored by the backend design mode.</p>
                <p>The UI only displays what the daemon reports — it does not generate or invent artifacts.</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
