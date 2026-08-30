import { useEffect, useState } from "react";
import { Icon } from "./Icon";
import type { SecurityFinding, ScannerInfo } from "./types";
import { daemon } from "./daemon";

const SEVERITY_META = {
  high: { color: "text-error", bg: "bg-error/10", icon: "warning", label: "High Severity" },
  medium: { color: "text-tertiary", bg: "bg-tertiary/10", icon: "error", label: "Medium Severity" },
  low: { color: "text-primary", bg: "bg-primary/10", icon: "info", label: "Low Severity" },
  info: { color: "text-primary", bg: "bg-primary/10", icon: "info", label: "Info" },
} as const;

export function SecurityView() {
  const [findings, setFindings] = useState<SecurityFinding[]>([]);
  const [scanners, setScanners] = useState<ScannerInfo[]>([]);
  const [selected, setSelected] = useState<SecurityFinding | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const [f, s] = await Promise.all([daemon.listSecurityFindings(), daemon.listScanners()]);
      if (cancelled) return;
      setFindings(f);
      setScanners(s);
      if (f.length > 0) setSelected(f[0]);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const count = (sev: SecurityFinding["severity"]) => findings.filter((f) => f.severity === sev).length;

  return (
    <main className="flex-1 overflow-y-auto p-6 md:p-8 flex gap-8">
      <div className="flex-1 max-w-5xl mx-auto space-y-8">
        <div>
          <h2 className="text-2xl font-semibold text-on-surface mb-2 tracking-tight">Security Audit Report</h2>
          <p className="text-on-surface-variant text-sm">
            {scanners.length > 0 ? `${scanners.length} scanners configured` : "Scanner status from daemon."}
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {(["high", "medium", "low"] as const).map((sev) => {
            const meta = SEVERITY_META[sev];
            return (
              <div key={sev} className="bg-background rounded-xl p-5 neo-raised border border-outline-variant/30">
                <div className="flex justify-between items-start mb-4">
                  <div className={`w-10 h-10 rounded-full ${meta.bg} flex items-center justify-center neo-pressed ${meta.color}`}>
                    <Icon name={meta.icon} fill />
                  </div>
                  <span className={`text-3xl font-bold ${meta.color} tracking-tighter`}>{count(sev)}</span>
                </div>
                <div>
                  <h3 className="font-medium text-on-surface">{meta.label}</h3>
                  <p className="text-xs text-on-surface-variant mt-1">{sev === "high" ? "Requires immediate action" : sev === "medium" ? "Review next sprint" : "Best practice updates"}</p>
                </div>
              </div>
            );
          })}
        </div>

        {selected && (
          <div className="bg-background rounded-2xl p-6 md:p-8 neo-raised">
            <div className="flex items-start justify-between mb-6">
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <span className={`px-2.5 py-1 rounded-md text-xs font-bold uppercase tracking-wider ${SEVERITY_META[selected.severity].color} ${SEVERITY_META[selected.severity].bg}`}>
                    {selected.severity}
                  </span>
                  {selected.file && (
                    <span className="text-xs text-on-surface-variant font-mono neo-pressed px-2 py-0.5 rounded bg-surface">{selected.file}</span>
                  )}
                </div>
                <h3 className="text-xl font-bold text-on-surface">{selected.title}</h3>
              </div>
            </div>
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
              <div className="lg:col-span-2 space-y-6">
                <div className="bg-surface rounded-xl p-5 neo-pressed">
                  <h4 className="text-sm font-semibold text-on-surface mb-3 flex items-center gap-2">
                    <Icon name="description" size={16} className="text-primary" />
                    Description
                  </h4>
                  <p className="text-sm text-on-surface-variant leading-relaxed">{selected.description}</p>
                </div>
                {selected.remediation && (
                  <div className="bg-surface rounded-xl p-5 neo-pressed">
                    <h4 className="text-sm font-semibold text-on-surface mb-3 flex items-center gap-2">
                      <Icon name="build" size={16} className="text-primary" />
                      Remediation
                    </h4>
                    <p className="text-sm text-on-surface-variant leading-relaxed">{selected.remediation}</p>
                  </div>
                )}
              </div>
              <div className="space-y-6">
                <div className="bg-surface rounded-xl p-5 neo-pressed">
                  <h4 className="text-sm font-semibold text-on-surface mb-3 flex items-center gap-2">
                    <Icon name="folder" size={16} className="text-primary" />
                    Affected Files
                  </h4>
                  {selected.file ? (
                    <div className="flex items-center justify-between text-sm p-2 bg-background rounded-lg neo-raised">
                      <span className="font-mono text-xs truncate">{selected.file}</span>
                      {selected.line && <span className="text-xs text-error font-medium">L: {selected.line}</span>}
                    </div>
                  ) : (
                    <p className="text-xs text-on-surface-variant">No file attributed.</p>
                  )}
                </div>
                <div className="bg-surface rounded-xl p-5 neo-pressed">
                  <h4 className="text-sm font-semibold text-on-surface mb-3">Scanner</h4>
                  <p className="text-sm text-on-surface-variant">{selected.scanner}</p>
                </div>
              </div>
            </div>
          </div>
        )}

        {findings.length > 0 && (
          <div className="bg-background rounded-2xl p-6 neo-raised">
            <h4 className="text-sm font-semibold text-on-surface mb-4">All Findings</h4>
            <div className="space-y-2">
              {findings.map((f) => (
                <button
                  key={f.id}
                  onClick={() => setSelected(f)}
                  className={`w-full flex items-center gap-3 p-3 rounded-xl text-left ${selected?.id === f.id ? "neo-pressed" : "bg-surface hover:bg-surface-variant/40"}`}
                >
                  <span className={`${SEVERITY_META[f.severity].color}`}>
                    <Icon name={SEVERITY_META[f.severity].icon} size={16} fill />
                  </span>
                  <span className="text-sm text-on-surface flex-1 truncate">{f.title}</span>
                  <span className="text-xs text-on-surface-variant">{f.scanner}</span>
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </main>
  );
}