import { useState, useEffect } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import { useTheme } from "./ThemeContext";
import type { SettingsState, ProviderInfo, DaemonStatus, ToolInfo, ScannerInfo, MemoryInfo } from "./types";

const SETTINGS_NAV = [
  { id: "providers", label: "Providers & Models", icon: "api" },
  { id: "tools", label: "Tools", icon: "handyman" },
  { id: "scanners", label: "Security Scanners", icon: "shield" },
  { id: "memory", label: "Memory", icon: "memory" },
  { id: "daemon", label: "Daemon", icon: "dns" },
  { id: "appearance", label: "Appearance", icon: "palette" },
  { id: "autonomy", label: "Autonomy Defaults", icon: "tune" },
];

type SettingsTab = (typeof SETTINGS_NAV)[number]["id"];

export function SettingsView() {
  const [tab, setTab] = useState<SettingsTab>("providers");
  const [settings, setSettings] = useState<SettingsState | null>(null);
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [scanners, setScanners] = useState<ScannerInfo[]>([]);
  const [memory, setMemory] = useState<MemoryInfo[]>([]);
  const [ollamaStatus, setOllamaStatus] = useState<{ running: boolean; models: string[] }>({ running: false, models: [] });
  const { theme, set: setTheme } = useTheme();

  useEffect(() => {
    (async () => {
      const [s, d, p, t, sc, m] = await Promise.all([
        daemon.getSettings(),
        daemon.health(),
        daemon.listProviders(),
        daemon.listTools(),
        daemon.listScanners(),
        daemon.listMemory(),
      ]);
      setSettings(s);
      setDaemonStatus(d);
      setProviders(p);
      setTools(t);
      setScanners(sc);
      setMemory(m);
    })();
  }, []);

  const updateSettings = (patch: Partial<SettingsState>) => {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    daemon.setSettings(next);
  };

  const [accountModal, setAccountModal] = useState<{ providerId: string; providerName: string } | null>(null);
  const [accountLabel, setAccountLabel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [organization, setOrganization] = useState("");
  const [project, setProject] = useState("");
  const [workspace, setWorkspace] = useState("");
  const [testResult, setTestResult] = useState<{ ok: boolean; message: string } | null>(null);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [providerSearch, setProviderSearch] = useState("");
  const [providerCategoryFilter, setProviderCategoryFilter] = useState<string | null>(null);
  const [rotateAccountId, setRotateAccountId] = useState<string | null>(null);
  const [rotateKey, setRotateKey] = useState("");
  const [rotateResult, setRotateResult] = useState<{ ok: boolean; message: string } | null>(null);

  const handleTestConnection = async () => {
    if (!accountModal || !apiKey) return;
    setTesting(true);
    setTestResult(null);
    const result = await daemon.testConnection(accountModal.providerId, apiKey, organization, project, workspace);
    setTestResult({ ok: result.ok, message: result.ok ? "Connection successful. Credential is valid." : result.error || "Connection failed." });
    setTesting(false);
  };

  const handleSaveAccount = async () => {
    if (!accountModal || !apiKey) return;
    setSaving(true);
    const result = await daemon.addAccount(accountModal.providerId, accountLabel || "Personal", apiKey, organization, project, workspace);
    if (result.ok) {
      setTestResult({ ok: true, message: "Connection successful. Account saved securely." });
    } else {
      setTestResult({ ok: false, message: result.error || "Authentication failed. The API key was rejected." });
    }
    setSaving(false);
    if (result.ok) {
      setTimeout(() => {
        setAccountModal(null);
        setAccountLabel("");
        setApiKey("");
        setOrganization("");
        setProject("");
        setWorkspace("");
        setTestResult(null);
        daemon.listProviders().then(setProviders);
      }, 1500);
    }
  };

  const handleRemoveAccount = async (accountId: string) => {
    await daemon.removeAccount(accountId);
    daemon.listProviders().then(setProviders);
  };

  const handleToggleEnabled = async (accountId: string, enabled: boolean) => {
    await daemon.setAccountEnabled(accountId, !enabled);
    daemon.listProviders().then(setProviders);
  };

  const handleRotate = async (accountId: string) => {
    if (!rotateKey) return;
    setRotateResult(null);
    const result = await daemon.rotateAccount(accountId, rotateKey);
    setRotateResult({ ok: result.ok, message: result.ok ? "Credential rotated successfully." : result.error || "Rotation failed." });
    if (result.ok) {
      setRotateAccountId(null);
      setRotateKey("");
      setRotateResult(null);
      daemon.listProviders().then(setProviders);
    }
  };

  useEffect(() => {
    if (tab === "providers") {
      let cancelled = false;
      const refresh = async () => {
        const o = await daemon.discoverOllama();
        if (!cancelled) {
          setOllamaStatus({ running: o.running, models: o.models.map((m) => m.name) });
        }
      };
      refresh();
      return () => {
        cancelled = true;
      };
    }
  }, [tab]);

  const handleRefreshOllama = async () => {
    const o = await daemon.refreshOllama();
    setOllamaStatus({ running: o.running, models: o.models.map((m) => m.name) });
  };

  return (
    <main className="flex-1 flex flex-col overflow-hidden">
      <div className="flex-1 overflow-y-auto p-8 flex gap-8">
        <aside className="w-64 flex-shrink-0">
          <h2 className="font-semibold text-lg mb-6 text-on-surface">Settings</h2>
          <nav className="flex flex-col gap-2">
            {SETTINGS_NAV.map((item) => (
              <button
                key={item.id}
                onClick={() => setTab(item.id)}
                className={`px-4 py-3 rounded-lg flex items-center gap-3 text-sm transition-colors ${
                  tab === item.id
                    ? "text-primary font-medium bg-surface-variant/30 neo-pressed"
                    : "text-on-surface-variant hover:bg-surface-variant/40"
                }`}
              >
                <Icon name={item.icon} size={16} />
                {item.label}
              </button>
            ))}
          </nav>
        </aside>

        <div className="flex-1 max-w-4xl space-y-8">
          {tab === "providers" && (
            <>
              <div className="mb-8">
                <h3 className="font-semibold text-2xl text-on-surface">Providers & Models</h3>
                <p className="text-on-surface-variant mt-2">Configure AI providers and manage model routing.</p>
              </div>

              {/* Search + filters */}
              <div className="neo-raised p-4 rounded-2xl mb-6 flex flex-col gap-3">
                <div className="relative">
                  <div className="absolute inset-y-0 left-4 flex items-center pointer-events-none text-outline">
                    <Icon name="search" size={18} />
                  </div>
                  <input
                    className="neo-input w-full py-3 pl-11 pr-4 rounded-xl text-sm"
                    placeholder="Search providers..."
                    value={providerSearch}
                    onChange={(e) => setProviderSearch(e.target.value)}
                  />
                </div>
                <div className="flex flex-wrap gap-2">
                  {[
                    { id: null, label: "All" },
                    { id: "free", label: "Free" },
                    { id: "free-tier", label: "Free Tier" },
                    { id: "paid", label: "Paid" },
                    { id: "local", label: "Local" },
                  ].map((f) => (
                    <button
                      key={f.id ?? "all"}
                      onClick={() => setProviderCategoryFilter(f.id)}
                      className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                        providerCategoryFilter === f.id
                          ? "bg-primary/10 text-primary neo-pressed"
                          : "neo-button text-on-surface-variant"
                      }`}
                    >
                      {f.label}
                    </button>
                  ))}
                </div>
              </div>

              {/* Provider cards */}
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-10">
                {providers
                  .filter((p) => (providerCategoryFilter ? p.category === providerCategoryFilter : true))
                  .filter((p) => p.name.toLowerCase().includes(providerSearch.toLowerCase()))
                  .map((p) => (
                  <div key={p.id} className="neo-raised p-6 rounded-2xl flex flex-col gap-4 relative overflow-hidden">
                    <div className="flex items-start justify-between">
                      <div className="w-10 h-10 rounded-full neo-pressed flex items-center justify-center text-primary">
                        <Icon name="smart_toy" size={20} />
                      </div>
                      <HealthBadge health={p.health} />
                    </div>
                    <div>
                      <h4 className="font-semibold text-on-surface">{p.name}</h4>
                      {p.description && <p className="text-xs text-on-surface-variant mt-0.5">{p.description}</p>}
                    </div>
                    <div className="text-xs text-on-surface-variant space-y-1">
                      <p>{p.model_count} model{p.model_count !== 1 ? "s" : ""}</p>
                      <p>{p.connected_accounts} account{p.connected_accounts !== 1 ? "s" : ""}</p>
                    </div>

                    {p.accounts.length > 0 && (
                      <div className="space-y-2">
                        {p.accounts.map((a) => (
                          <div key={a.id} className="neo-pressed rounded-xl p-3 flex flex-col gap-2">
                            <div className="flex items-center justify-between gap-2">
                              <span className="text-xs font-semibold text-on-surface truncate">{a.label}</span>
                              <span className={`text-[10px] px-2 py-0.5 rounded-full font-medium ${
                                a.enabled ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" : "bg-on-surface-variant/10 text-on-surface-variant"
                              }`}>
                                {a.enabled ? "Enabled" : "Disabled"}
                              </span>
                            </div>
                            <div className="text-[11px] text-on-surface-variant font-mono truncate">{a.credential_masked}</div>
                            {(a.organization || a.project || a.workspace) && (
                              <div className="text-[11px] text-on-surface-variant truncate">
                                {[a.organization, a.project, a.workspace].filter(Boolean).join(" · ")}
                              </div>
                            )}
                            {a.last_failure && (
                              <div className="text-[11px] text-red-600 dark:text-red-400 font-medium">Last failure recorded</div>
                            )}
                            <div className="flex flex-wrap gap-1.5">
                              <button
                                onClick={() => handleToggleEnabled(a.id, a.enabled)}
                                className="neo-button px-2 py-1 rounded-md text-[11px] text-primary font-medium"
                              >
                                {a.enabled ? "Disable" : "Enable"}
                              </button>
                              <button
                                onClick={() => { setRotateAccountId(a.id); setRotateKey(""); setRotateResult(null); }}
                                className="neo-button px-2 py-1 rounded-md text-[11px] text-primary font-medium"
                              >
                                Rotate
                              </button>
                              <button
                                onClick={() => handleRemoveAccount(a.id)}
                                className="neo-button px-2 py-1 rounded-md text-[11px] text-red-600 dark:text-red-400 font-medium"
                              >
                                Remove
                              </button>
                            </div>
                            {rotateAccountId === a.id && (
                              <div className="flex flex-col gap-2 mt-1">
                                <input
                                  type="password"
                                  className="neo-input w-full py-2 px-3 rounded-lg text-xs"
                                  placeholder="New API key"
                                  value={rotateKey}
                                  onChange={(e) => setRotateKey(e.target.value)}
                                />
                                {rotateResult && (
                                  <p className={`text-[11px] font-medium ${rotateResult.ok ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}>
                                    {rotateResult.message}
                                  </p>
                                )}
                                <div className="flex gap-2">
                                  <button
                                    onClick={() => handleRotate(a.id)}
                                    disabled={!rotateKey}
                                    className="px-3 py-1.5 rounded-md bg-primary text-on-primary text-[11px] font-medium disabled:opacity-50"
                                  >
                                    Confirm Rotate
                                  </button>
                                  <button
                                    onClick={() => { setRotateAccountId(null); setRotateKey(""); setRotateResult(null); }}
                                    className="neo-button px-3 py-1.5 rounded-md text-[11px] text-on-surface-variant font-medium"
                                  >
                                    Cancel
                                  </button>
                                </div>
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    )}

                    <div className="flex gap-2 mt-auto">
                      <button
                        onClick={() => setAccountModal({ providerId: p.id, providerName: p.name })}
                        className="flex-1 py-2 rounded-lg neo-button text-xs text-primary font-medium"
                      >
                        + Add Account
                      </button>
                      {p.credential_url && (
                        <a
                          href={p.credential_url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-xs text-primary hover:underline self-center"
                        >
                          Get API key →
                        </a>
                      )}
                    </div>
                  </div>
                ))}
              </div>

              {/* No model routes configured */}
              {providers.every((p) => p.connected_accounts === 0) && !ollamaStatus.running && (
                <div className="neo-raised p-8 rounded-2xl mb-10 text-center">
                  <div className="w-14 h-14 rounded-full neo-pressed mx-auto mb-4 flex items-center justify-center text-primary">
                    <Icon name="route" size={26} />
                  </div>
                  <h4 className="font-semibold text-lg text-on-surface">No model routes configured</h4>
                  <p className="text-sm text-on-surface-variant mt-2 max-w-md mx-auto">
                    AgentCode needs at least one usable model route. Connect a provider account or start a local model to begin.
                  </p>
                  <div className="flex justify-center gap-3 mt-6">
                    <button
                      onClick={() => { const first = providers[0]; if (first) setAccountModal({ providerId: first.id, providerName: first.name }); }}
                      className="px-6 py-3 rounded-xl bg-primary text-on-primary text-sm font-medium hover:brightness-110 transition-all"
                    >
                      Configure Provider
                    </button>
                  </div>
                </div>
              )}

              {/* Local Models */}
              <div className="neo-raised p-6 rounded-2xl">
                <div className="flex items-center justify-between mb-4">
                  <div>
                    <h4 className="font-semibold text-lg text-on-surface">Local Models</h4>
                    <p className="text-sm text-on-surface-variant mt-1">Ollama and LM Studio</p>
                  </div>
                  <button onClick={handleRefreshOllama} className="neo-button px-4 py-2 rounded-lg text-sm text-primary font-medium flex items-center gap-2">
                    <Icon name="refresh" size={16} />
                    Refresh
                  </button>
                </div>
                <div className={ollamaStatus.running ? "text-emerald-600 dark:text-emerald-400 text-sm flex items-center gap-2 mb-3" : "text-on-surface-variant text-sm flex items-center gap-2 mb-3"}>
                  <span className={`w-2 h-2 rounded-full ${ollamaStatus.running ? "bg-emerald-500" : "bg-on-surface-variant"}`} />
                  {ollamaStatus.running ? "Running" : "Not running"}
                </div>
                {ollamaStatus.models.length > 0 && (
                  <div className="flex flex-wrap gap-2">
                    {ollamaStatus.models.map((name) => (
                      <span key={name} className="neo-button text-xs px-3 py-1.5 rounded-lg text-on-surface-variant font-mono">
                        {name}
                      </span>
                    ))}
                  </div>
                )}
                {!ollamaStatus.running && (
                  <p className="text-xs text-on-surface-variant mt-2">Ollama is not available. Install and start Ollama to use local models.</p>
                )}
              </div>
            </>
          )}

          {tab === "tools" && (
            <div className="neo-raised p-8 rounded-2xl">
              <h3 className="font-semibold text-lg text-on-surface mb-4">Tools</h3>
              <div className="space-y-3">
                {tools.length === 0 && <p className="text-sm text-on-surface-variant">Tool availability from daemon.</p>}
                {tools.map((t) => (
                  <div key={t.id} className="neo-pressed p-4 rounded-xl flex items-center justify-between">
                    <span className="text-sm font-medium text-on-surface">{t.name}</span>
                    <span className={`text-xs px-2 py-1 rounded-full font-medium ${
                      t.state === "available" ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" :
                      t.state === "unavailable" ? "bg-red-500/10 text-red-600" : "bg-primary/10 text-primary"
                    }`}>
                      {t.state}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {tab === "scanners" && (
            <div className="neo-raised p-8 rounded-2xl">
              <h3 className="font-semibold text-lg text-on-surface mb-4">Security Scanners</h3>
              <div className="space-y-3">
                {scanners.length === 0 && <p className="text-sm text-on-surface-variant">Scanner status from daemon.</p>}
                {scanners.map((s) => (
                  <div key={s.id} className="neo-pressed p-4 rounded-xl flex items-center justify-between">
                    <span className="text-sm font-medium text-on-surface">{s.name}</span>
                    <span className={`text-xs px-2 py-1 rounded-full font-medium ${
                      s.state === "passed" ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" :
                      s.state === "available" ? "bg-primary/10 text-primary" :
                      s.state === "running" ? "bg-amber-500/10 text-amber-600" :
                      "bg-on-surface-variant/10 text-on-surface-variant"
                    }`}>
                      {s.state}{s.required ? " · Required" : " · Optional"}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {tab === "memory" && (
            <div className="neo-raised p-8 rounded-2xl">
              <h3 className="font-semibold text-lg text-on-surface mb-4">Memory</h3>
              {memory.length === 0 ? (
                <p className="text-sm text-on-surface-variant">Memory facts from daemon.</p>
              ) : (
                <div className="space-y-3">
                  {memory.map((m) => (
                    <div key={m.id} className="neo-pressed p-4 rounded-xl">
                      <p className="text-sm font-medium text-on-surface">{m.statement}</p>
                      <p className="text-xs text-on-surface-variant mt-1">{m.label} · {m.freshness}</p>
                      <p className="text-[11px] text-on-surface-variant">{m.source} · {m.date}</p>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {tab === "daemon" && daemonStatus && (
            <div className="neo-raised p-8 rounded-2xl">
              <h3 className="font-semibold text-lg text-on-surface mb-4">Daemon</h3>
              <div className="grid grid-cols-2 gap-4 mb-6">
                {[
                  ["Status", daemonStatus.state, daemonStatus.state === "running" ? "text-emerald-600" : "text-red-600"],
                  ["Instance ID", daemonStatus.instance_id || "—"],
                  ["Uptime", daemonStatus.uptime || "—"],
                  ["Socket", daemonStatus.socket || "—"],
                  ["Database", daemonStatus.database || "—"],
                  ["Recovered Sessions", String(daemonStatus.recovered_sessions)],
                ].map(([label, value, cls]) => (
                  <div key={label} className="neo-pressed p-3 rounded-xl flex items-center justify-between">
                    <span className="text-sm text-on-surface-variant">{label}</span>
                    <span className={`text-sm font-code text-on-surface ${cls || ""}`}>{value}</span>
                  </div>
                ))}
              </div>
              <button className="neo-button px-6 py-2 rounded-lg text-sm text-primary font-medium">Reconnect</button>
            </div>
          )}

          {tab === "appearance" && (
            <div className="neo-raised p-8 rounded-2xl">
              <h3 className="font-semibold text-lg text-on-surface mb-4">Appearance</h3>
              <div className="neo-pressed rounded-xl p-2 inline-flex">
                {(["light", "dark", "system"] as const).map((t) => (
                  <button
                    key={t}
                    onClick={() => {
                      if (t !== "system") setTheme(t);
                      updateSettings({ appearance: t });
                    }}
                    className={`px-5 py-2 rounded-lg text-sm font-medium flex items-center gap-2 transition-colors ${
                      (t === "light" && theme === "light") || (t === "dark" && theme === "dark") || (t === "system" && settings?.appearance === "system")
                        ? "bg-surface text-primary neo-raised"
                        : "text-on-surface-variant hover:text-on-surface"
                    }`}
                  >
                    <Icon name={t === "light" ? "light_mode" : t === "dark" ? "dark_mode" : "desktop_windows"} size={16} />
                    {t.charAt(0).toUpperCase() + t.slice(1)}
                  </button>
                ))}
              </div>
              <div className="mt-8 space-y-4">
                <label className="flex items-center justify-between neo-pressed p-4 rounded-xl cursor-pointer">
                  <span className="text-sm text-on-surface">Notifications</span>
                  <input
                    type="checkbox"
                    checked={settings?.notifications_enabled ?? true}
                    onChange={(e) => updateSettings({ notifications_enabled: e.target.checked })}
                    className="rounded border-outline-variant text-primary focus:ring-primary"
                  />
                </label>
                <label className="flex items-center justify-between neo-pressed p-4 rounded-xl cursor-pointer">
                  <span className="text-sm text-on-surface">Completion Sound</span>
                  <input
                    type="checkbox"
                    checked={settings?.completion_sound ?? true}
                    onChange={(e) => updateSettings({ completion_sound: e.target.checked })}
                    className="rounded border-outline-variant text-primary focus:ring-primary"
                  />
                </label>
                <label className="flex items-center justify-between neo-pressed p-4 rounded-xl cursor-pointer">
                  <span className="text-sm text-on-surface">Reduced Motion</span>
                  <input
                    type="checkbox"
                    checked={settings?.reduced_motion ?? false}
                    onChange={(e) => updateSettings({ reduced_motion: e.target.checked })}
                    className="rounded border-outline-variant text-primary focus:ring-primary"
                  />
                </label>
              </div>
            </div>
          )}

          {tab === "autonomy" && (
            <div className="neo-raised p-8 rounded-2xl">
              <h3 className="font-semibold text-lg text-on-surface mb-4">Autonomy Defaults</h3>
              <div className="space-y-6">
                <div>
                  <label className="block text-sm text-on-surface-variant mb-2">Default Model Routing</label>
                  <select
                    value={settings?.routing_profile ?? "free_first"}
                    onChange={(e) => updateSettings({ routing_profile: e.target.value as SettingsState["routing_profile"] })}
                    className="neo-input w-full py-3 px-4 rounded-xl text-sm text-on-surface"
                  >
                    <option value="free_first">Free First</option>
                    <option value="local_first">Local First</option>
                    <option value="quality_first">Quality First</option>
                    <option value="paid_allowed">Paid Allowed</option>
                    <option value="offline">Offline</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm text-on-surface-variant mb-2">Preferred Model (optional)</label>
                  <input
                    className="neo-input w-full py-3 px-4 rounded-xl text-sm text-on-surface"
                    placeholder="e.g., gpt-4-turbo"
                    value={settings?.preferred_model ?? ""}
                    onChange={(e) => updateSettings({ preferred_model: e.target.value || undefined })}
                  />
                </div>
                <div>
                  <label className="block text-sm text-on-surface-variant mb-2">Budget Limit (micros, optional)</label>
                  <input
                    type="number"
                    className="neo-input w-full py-3 px-4 rounded-xl text-sm text-on-surface"
                    placeholder="e.g., 5000"
                    value={settings?.budget_limit_micros ?? ""}
                    onChange={(e) => updateSettings({ budget_limit_micros: e.target.value ? Number(e.target.value) : undefined })}
                  />
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Add Account Modal */}
      {accountModal && (
        <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={() => { setAccountModal(null); setTestResult(null); }}>
          <div className="bg-surface rounded-2xl p-8 max-w-lg w-full mx-4 neo-raised max-h-[90vh] overflow-y-auto" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-xl font-semibold text-on-surface mb-6">Add Account</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-xs text-on-surface-variant mb-1">Provider</label>
                <p className="text-sm text-on-surface font-medium">{accountModal.providerName}</p>
              </div>
              <div>
                <label className="block text-xs text-on-surface-variant mb-1">Account Name</label>
                <input className="neo-input w-full py-3 px-4 rounded-xl text-sm" placeholder="Personal" value={accountLabel} onChange={(e) => setAccountLabel(e.target.value)} />
              </div>
              <div>
                <label className="block text-xs text-on-surface-variant mb-1">API Key</label>
                <input className="neo-input w-full py-3 px-4 rounded-xl text-sm" type="password" placeholder="••••••••••••" value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
              </div>
              <div>
                <label className="block text-xs text-on-surface-variant mb-1">Organization (optional)</label>
                <input className="neo-input w-full py-3 px-4 rounded-xl text-sm" placeholder="org-id" value={organization} onChange={(e) => setOrganization(e.target.value)} />
              </div>
              <div>
                <label className="block text-xs text-on-surface-variant mb-1">Project (optional)</label>
                <input className="neo-input w-full py-3 px-4 rounded-xl text-sm" placeholder="project-id" value={project} onChange={(e) => setProject(e.target.value)} />
              </div>
              <div>
                <label className="block text-xs text-on-surface-variant mb-1">Workspace (optional)</label>
                <input className="neo-input w-full py-3 px-4 rounded-xl text-sm" placeholder="workspace-id" value={workspace} onChange={(e) => setWorkspace(e.target.value)} />
              </div>

              {testResult && (
                <div className={`p-4 rounded-xl ${testResult.ok ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" : "bg-red-500/10 text-red-600 dark:text-red-400"} text-sm font-medium`}>
                  {testResult.ok ? (
                    <span className="flex items-center gap-2"><Icon name="check_circle" size={16} fill /> {testResult.message}</span>
                  ) : (
                    <span className="flex items-center gap-2"><Icon name="error" size={16} fill /> {testResult.message}</span>
                  )}
                </div>
              )}

              <div className="flex flex-wrap justify-end gap-3 mt-6">
                <button
onClick={() => { setAccountModal(null); setTestResult(null); }}
                  className="neo-button px-6 py-3 rounded-xl text-sm text-on-surface-variant font-medium"
                >
                  Cancel
                </button>
                <button
                  onClick={handleTestConnection}
                  disabled={testing || !apiKey}
                  className="neo-button px-6 py-3 rounded-xl text-sm text-primary font-medium disabled:opacity-50"
                >
                  {testing ? "Testing..." : "Test Connection"}
                </button>
                <button
                  onClick={handleSaveAccount}
                  disabled={saving || !apiKey}
                  className="px-6 py-3 rounded-xl bg-primary text-on-primary text-sm font-medium hover:brightness-110 transition-all disabled:opacity-50"
                >
                  {saving ? "Saving..." : "Save Account"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}

function HealthBadge({ health }: { health: string }) {
  const meta: Record<string, { label: string; cls: string }> = {
    healthy: { label: "Healthy", cls: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" },
    degraded: { label: "Degraded", cls: "bg-amber-500/10 text-amber-600 dark:text-amber-400" },
    rate_limited: { label: "Rate Limited", cls: "bg-orange-500/10 text-orange-600 dark:text-orange-400" },
    auth_failed: { label: "Auth Failed", cls: "bg-red-500/10 text-red-600 dark:text-red-400" },
    unavailable: { label: "Unavailable", cls: "bg-on-surface-variant/10 text-on-surface-variant" },
    disabled: { label: "Disabled", cls: "bg-on-surface-variant/10 text-on-surface-variant" },
  };
  const m = meta[health] || meta.unavailable;
  return (
    <span className={`px-2.5 py-1 rounded-full text-xs font-medium ${m.cls}`}>
      {m.label}
    </span>
  );
}