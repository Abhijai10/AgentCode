import { useState } from "react";
import { Icon } from "./Icon";
import { daemon } from "./daemon";
import type { Project } from "./ProjectContext";

export function ProjectModal({
  mode,
  onClose,
  onOpened,
}: {
  mode: "open" | "new" | "choose";
  onClose(): void;
  onOpened(p: Project): void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [location, setLocation] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [screen, setScreen] = useState<"choose" | "new">(mode === "new" ? "new" : "choose");

  const handleChooseLocation = async () => {
    setBusy(true);
    setError(null);
    const loc = await daemon.pickProjectLocation();
    setBusy(false);
    if (loc) setLocation(loc);
  };

  const handleCreate = async () => {
    if (!location) return;
    setBusy(true);
    setError(null);
    const path = await daemon.createProjectFolder(location, name);
    setBusy(false);
    if (path) {
      onOpened({ path, name: name.trim() });
    } else {
      setError("Could not create the project folder.");
    }
  };

  const handleOpenExisting = async () => {
    setBusy(true);
    setError(null);
    const path = await daemon.pickProjectFolder();
    setBusy(false);
    if (path) {
      const parts = path.split("/").filter(Boolean);
      onOpened({ path, name: parts[parts.length - 1] || path });
    }
  };

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={onClose}>
      <div
        className="bg-surface rounded-2xl p-8 max-w-md w-full mx-4 neo-raised"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-xl font-semibold text-on-surface mb-6">
          {screen === "new" ? "Start a New Project" : mode === "open" ? "Open a Project" : "Get Started"}
        </h3>

        {screen === "choose" ? (
          <div className="space-y-4">
            <button
              onClick={() => setScreen("new")}
              className="w-full neo-button rounded-2xl p-6 flex items-start gap-4 text-left hover:text-primary transition-colors"
            >
              <div className="w-12 h-12 rounded-full neo-pressed flex items-center justify-center text-primary shrink-0">
                <Icon name="create_new_folder" size={24} />
              </div>
              <div>
                <h4 className="font-semibold text-on-surface">Start from Scratch</h4>
                <p className="text-sm text-on-surface-variant mt-1">Create a new empty project. Choose a location and give it a name.</p>
              </div>
            </button>
            <button
              onClick={handleOpenExisting}
              disabled={busy}
              className="w-full neo-button rounded-2xl p-6 flex items-start gap-4 text-left hover:text-primary transition-colors disabled:opacity-60"
            >
              <div className="w-12 h-12 rounded-full neo-pressed flex items-center justify-center text-primary shrink-0">
                <Icon name="folder_open" size={24} />
              </div>
              <div>
                <h4 className="font-semibold text-on-surface">Open Existing Project</h4>
                <p className="text-sm text-on-surface-variant mt-1">Browse to an existing folder on your machine.</p>
              </div>
            </button>
            {error && (
              <p className="text-sm text-red-600 dark:text-red-400 font-medium flex items-center gap-2">
                <Icon name="error" size={16} fill /> {error}
              </p>
            )}
          </div>
        ) : (
          <div className="space-y-4">
            <div>
              <label className="block text-xs text-on-surface-variant mb-1">Project Location</label>
              <div className="flex gap-2 items-center">
                <div className="flex-1 neo-pressed rounded-xl px-4 py-3 text-sm text-on-surface truncate">
                  {location || "No location selected"}
                </div>
                <button
                  onClick={handleChooseLocation}
                  disabled={busy}
                  className="neo-button px-4 py-3 rounded-xl text-sm text-primary font-medium flex items-center gap-2"
                >
                  <Icon name="folder" size={16} />
                  Browse
                </button>
              </div>
            </div>
            <div>
              <label className="block text-xs text-on-surface-variant mb-1">Project Name</label>
              <input
                className="neo-input w-full py-3 px-4 rounded-xl text-sm"
                placeholder="my-project"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>
            {error && (
              <p className="text-sm text-red-600 dark:text-red-400 font-medium flex items-center gap-2">
                <Icon name="error" size={16} fill /> {error}
              </p>
            )}
            <div className="flex justify-between items-center gap-3 mt-6">
              <button
                onClick={() => setScreen("choose")}
                className="neo-button px-5 py-3 rounded-xl text-sm text-on-surface-variant font-medium"
              >
                Back
              </button>
              <div className="flex gap-3">
                <button onClick={onClose} className="neo-button px-6 py-3 rounded-xl text-sm text-on-surface-variant font-medium">
                  Cancel
                </button>
                <button
                  onClick={handleCreate}
                  disabled={busy || !location || !name.trim()}
                  className="px-6 py-3 rounded-xl bg-primary text-on-primary text-sm font-medium hover:brightness-110 transition-all disabled:opacity-50"
                >
                  {busy ? "Creating..." : "Create Project"}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}