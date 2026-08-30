export function Footer({ daemonConnected, modelLabel, tasks }: { daemonConnected: boolean; modelLabel: string; tasks: string }) {
  return (
    <footer className="h-8 shrink-0 border-t border-outline-variant/40 bg-surface-container-lowest/80 backdrop-blur-md flex items-center justify-between px-6 text-xs dark:border-white/5">
      <span className="text-on-surface-variant">AgentCode © 2024</span>
      <div className="flex items-center gap-5 text-on-surface-variant">
        <span className="flex items-center gap-1.5">
          <span className={`w-1.5 h-1.5 rounded-full ${daemonConnected ? "bg-emerald-500" : "bg-red-500"}`} />
          {daemonConnected ? "Connected" : "Disconnected"}
        </span>
        <span>{modelLabel}</span>
        <span>{tasks}</span>
      </div>
    </footer>
  );
}