import { createContext, useContext, useState, useEffect, type ReactNode } from "react";
import { daemon } from "./daemon";

type Theme = "light" | "dark";
type AppearancePref = "light" | "dark" | "system";

interface ThemeContextValue {
  theme: Theme;
  appearance: AppearancePref;
  toggle: () => void;
  set: (t: AppearancePref) => void;
}

const ThemeContext = createContext<ThemeContextValue>({
  theme: "light",
  appearance: "light",
  toggle: () => {},
  set: () => {},
});

function getInitialAppearance(): AppearancePref {
  if (typeof window === "undefined") return "light";
  const stored = localStorage.getItem("agentcode-theme");
  if (stored === "dark" || stored === "light" || stored === "system") return stored;
  return "light";
}

function systemPrefersDark(): boolean {
  return typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [appearance, setAppearance] = useState<AppearancePref>(getInitialAppearance);
  const [systemDark, setSystemDark] = useState<boolean>(systemPrefersDark);
  const theme: Theme = appearance === "system" ? (systemDark ? "dark" : "light") : appearance;

  // Load the persisted daemon appearance setting on mount (the real app setting).
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const settings = await daemon.getSettings();
      if (cancelled) return;
      if (settings.appearance === "dark" || settings.appearance === "light" || settings.appearance === "system") {
        setAppearance(settings.appearance);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Follow OS appearance changes when in "system" mode.
  useEffect(() => {
    if (appearance !== "system") return;
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    mql.addEventListener("change", onChange);
    setSystemDark(mql.matches);
    return () => mql.removeEventListener("change", onChange);
  }, [appearance]);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    localStorage.setItem("agentcode-theme", appearance);
    daemon.setSettings({ appearance });
  }, [theme, appearance]);

  const toggle = () => setAppearance((t) => (t === "dark" || (t === "system" && systemDark) ? "light" : "dark"));
  const set = (t: AppearancePref) => setAppearance(t);

  return (
    <ThemeContext.Provider value={{ theme, appearance, toggle, set }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  return useContext(ThemeContext);
}