import { Icon } from "./Icon";
import { useTheme } from "./ThemeContext";

export function TopBar({
  project,
  branch,
  onNewMission,
  onOpenProject,
}: {
  project: string;
  branch: string;
  onNewMission(): void;
  onOpenProject(): void;
}) {
  const { theme, toggle } = useTheme();
  return (
    <header className="h-16 shrink-0 flex justify-between items-center px-6 border-b border-outline-variant/40 bg-surface/80 backdrop-blur-md dark:border-white/5">
      <button onClick={onOpenProject} className="flex items-center gap-2 hover:text-primary transition-colors group">
        <span className="text-sm text-on-surface-variant font-medium group-hover:text-primary">{project}</span>
        <Icon name="chevron_right" size={16} className="text-outline" />
        <span className="text-sm text-primary font-bold">{branch}</span>
      </button>
      <div className="flex items-center gap-3">
        <button onClick={onOpenProject} title="Open project" className="w-9 h-9 rounded-full neo-button flex items-center justify-center text-on-surface-variant hover:text-primary transition-all duration-150">
          <Icon name="folder_open" size={18} />
        </button>
        <button onClick={toggle} title="Toggle theme" className="w-9 h-9 rounded-full neo-button flex items-center justify-center text-on-surface-variant hover:text-primary transition-all duration-150">
          <Icon name={theme === "dark" ? "light_mode" : "dark_mode"} size={18} />
        </button>
        <button className="w-9 h-9 rounded-full neo-button flex items-center justify-center text-on-surface-variant hover:text-primary transition-all duration-150">
          <Icon name="notifications" size={18} />
        </button>
        <button className="w-9 h-9 rounded-full neo-button flex items-center justify-center text-on-surface-variant hover:text-primary transition-all duration-150">
          <Icon name="account_circle" size={18} />
        </button>
        <button onClick={onNewMission} className="neo-button rounded-lg px-4 py-2 text-primary font-semibold text-sm flex items-center gap-2 active:scale-[0.98]">
          <Icon name="add" size={18} />
          New Mission
        </button>
      </div>
    </header>
  );
}