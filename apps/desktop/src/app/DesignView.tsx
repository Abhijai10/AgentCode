import { useEffect, useState } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { DesignSession } from "./types";

export function DesignView() {
  const [sessions, setSessions] = useState<DesignSession[]>([]);

  useEffect(() => {
    let cancelled = false;
    const refresh = async () => {
      const s = await daemon.designListSessions();
      if (!cancelled) setSessions(s);
    };
    refresh();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="flex-1 flex flex-col overflow-hidden">
      <div className="p-6 flex flex-col gap-6 overflow-y-auto flex-1">
        <div className="neo-raised rounded-2xl p-8 flex flex-col items-center text-center max-w-2xl mx-auto w-full">
          <div className="w-16 h-16 rounded-2xl neo-raised mb-4 flex items-center justify-center text-primary">
            <Icon name="design_services" size={30} />
          </div>
          <h2 className="text-2xl font-semibold text-on-surface mb-2">Design Studio</h2>
          <p className="text-sm text-on-surface-variant max-w-md mx-auto">
            Not available in this build — the daemon IPC does not expose a Design Studio session contract.
            Submit a coding mission from Home to do goal-driven work instead.
          </p>
          <div className="neo-pressed rounded-xl p-4 mt-6 text-xs text-on-surface-variant">
            {sessions.length > 0
              ? `${sessions.length} design session(s) recorded by the backend.`
              : "No design sessions recorded by the backend."}
          </div>
        </div>
      </div>
    </main>
  );
}
