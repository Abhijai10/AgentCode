import { useEffect, useState } from "react";
import { Icon } from "./Icon";
import type { DaemonStatus, ProviderInfo } from "./types";
import type { Project } from "./ProjectContext";
import { daemon } from "./daemon";

export function SecurityView({
  project,
  onOpenSettings,
}: {
  project: Project | null;
  onOpenSettings(): void;
}) {
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [ollamaRunning, setOllamaRunning] = useState<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const [d, p, o] = await Promise.all([
        daemon.health(),
        daemon.listProviders(),
        daemon.discoverOllama(),
      ]);
      if (cancelled) return;
      setDaemonStatus(d);
      setProviders(p);
      setOllamaRunning(o.running);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const connected = daemonStatus?.state === "running";
  const configuredAccounts = providers.reduce((n, p) => n + p.connected_accounts, 0);

  return (
    <main className="flex-1 overflow-y-auto p-6 md:p-8">
      <div className="max-w-5xl mx-auto flex flex-col gap-6">
        <div>
          <h2 className="text-2xl font-semibold text-on-surface mb-2 tracking-tight">Security</h2>
          <p className="text-on-surface-variant text-sm">AgentCode security posture from the daemon.</p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          <div className="bg-background rounded-xl p-5 neo-raised border border-outline-variant/30">
            <div className="flex justify-between items-start mb-3">
              <div className={`w-10 h-10 rounded-full flex items-center justify-center neo-pressed ${connected ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" : "bg-red-500/10 text-red-600 dark:text-red-400"}`}>
                <Icon name={connected ? "verified_user" : "dns_off"} fill />
              </div>
              <span className={`text-xs px-2 py-1 rounded-full font-medium ${connected ? "bg-emerald-500/10 text-emerald-600" : "bg-red-500/10 text-red-600"}`}>
                {connected ? "Protected" : "Unavailable"}
              </span>
            </div>
            <h3 className="font-medium text-on-surface">Daemon</h3>
            <p className="text-xs text-on-surface-variant mt-1">
              {connected ? "Connected and enforcing backend policy." : "The AgentCode daemon is not responding."}
            </p>
          </div>

          <div className="bg-background rounded-xl p-5 neo-raised border border-outline-variant/30">
            <div className="flex justify-between items-start mb-3">
              <div className="w-10 h-10 rounded-full flex items-center justify-center neo-pressed text-primary bg-primary/10">
                <Icon name="folder" fill />
              </div>
              <span className="text-xs px-2 py-1 rounded-full font-medium bg-primary/10 text-primary">
                {project ? "Bound" : "Unbound"}
              </span>
            </div>
            <h3 className="font-medium text-on-surface">Workspace Isolation</h3>
            <p className="text-xs text-on-surface-variant mt-1">
              {project
                ? `Project "${project.name}" is the only writable workspace root.`
                : "No project open; nothing is bound to the sandbox."}
            </p>
            {project && (
              <p className="text-[11px] text-on-surface-variant font-mono mt-2 truncate">{project.path}</p>
            )}
          </div>

          <div className="bg-background rounded-xl p-5 neo-raised border border-outline-variant/30">
            <div className="flex justify-between items-start mb-3">
              <div className="w-10 h-10 rounded-full flex items-center justify-center neo-pressed text-primary bg-primary/10">
                <Icon name="route" fill />
              </div>
              <span className="text-xs px-2 py-1 rounded-full font-medium bg-primary/10 text-primary">
                {ollamaRunning === null ? "Checking" : ollamaRunning ? "Local" : "Remote"}
              </span>
            </div>
            <h3 className="font-medium text-on-surface">Provider Mode</h3>
            <p className="text-xs text-on-surface-variant mt-1">
              {ollamaRunning === null
                ? "Checking local provider availability…"
                : ollamaRunning
                  ? "A local model server is available; privacy-class LocalOnly routing is possible."
                  : "No local model server detected; only explicitly configured external providers are eligible."}
            </p>
          </div>

          <div className="bg-background rounded-xl p-5 neo-raised border border-outline-variant/30">
            <div className="flex justify-between items-start mb-3">
              <div className="w-10 h-10 rounded-full flex items-center justify-center neo-pressed text-primary bg-primary/10">
                <Icon name="key" fill />
              </div>
              <span className={`text-xs px-2 py-1 rounded-full font-medium ${
                configuredAccounts > 0 ? "bg-emerald-500/10 text-emerald-600" : "bg-amber-500/10 text-amber-600"
              }`}>
                {configuredAccounts > 0 ? "Configured" : "Not configured"}
              </span>
            </div>
            <h3 className="font-medium text-on-surface">Credential Configuration</h3>
            <p className="text-xs text-on-surface-variant mt-1">
              {configuredAccounts > 0
                ? `${configuredAccounts} provider account${configuredAccounts !== 1 ? "s" : ""} stored in the backend secret store. Secrets never enter the UI.`
                : "No provider accounts configured. Credentials are stored only in the backend secret store."}
            </p>
          </div>

          <div className="bg-background rounded-xl p-5 neo-raised border border-outline-variant/30">
            <div className="flex justify-between items-start mb-3">
              <div className="w-10 h-10 rounded-full flex items-center justify-center neo-pressed text-primary bg-primary/10">
                <Icon name="security" fill />
              </div>
              <span className="text-xs px-2 py-1 rounded-full font-medium bg-on-surface-variant/10 text-on-surface-variant">
                Unavailable
              </span>
            </div>
            <h3 className="font-medium text-on-surface">Sandbox Status</h3>
            <p className="text-xs text-on-surface-variant mt-1">
              The daemon IPC does not expose a live sandbox/isolated-execution status in this build.
              Tool execution uses the backend&apos;s macOS sandbox under the hood.
            </p>
          </div>

          <div className="bg-background rounded-xl p-5 neo-raised border border-outline-variant/30">
            <div className="flex justify-between items-start mb-3">
              <div className="w-10 h-10 rounded-full flex items-center justify-center neo-pressed text-primary bg-primary/10">
                <Icon name="shield" fill />
              </div>
              <span className="text-xs px-2 py-1 rounded-full font-medium bg-on-surface-variant/10 text-on-surface-variant">
                Unavailable
              </span>
            </div>
            <h3 className="font-medium text-on-surface">Scanner Findings</h3>
            <p className="text-xs text-on-surface-variant mt-1">
              The daemon IPC does not expose a security-findings report in this build. Scanner
              execution is owned by the backend security subsystem.
            </p>
          </div>
        </div>

        <div className="neo-pressed rounded-xl p-4 text-sm text-on-surface-variant flex items-start gap-2">
          <Icon name="info" size={16} className="text-primary mt-0.5" />
          <div>
            <p className="text-on-surface font-medium mb-1">Boundaries</p>
            <p>
              This view shows only real state the backend exposes. Secret values are never
              displayed. Sandbox and scanner status are marked Unavailable because the daemon IPC
              does not expose them yet — nothing is guessed.
            </p>
          </div>
        </div>

        <div className="flex gap-3">
          <button
            onClick={onOpenSettings}
            className="neo-button px-5 py-2.5 rounded-xl text-sm text-primary font-medium flex items-center gap-2"
          >
            <Icon name="settings" size={16} />
            Open Settings
          </button>
        </div>
      </div>
    </main>
  );
}
