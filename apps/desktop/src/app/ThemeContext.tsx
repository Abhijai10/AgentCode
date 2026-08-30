import { createContext, useContext, useState, useEffect, type ReactNode } from "react";
import { daemon } from "./daemon";

type Theme = "light" | "dark";

interface ThemeContextValue {
  theme: Theme;
  toggle: () => void;
  set: (t: Theme) => void;
}

const ThemeContext = createContext<ThemeContextValue>({
  theme: "light",
  toggle: () => {},
  set: () => {},
});

function getInitialTheme(): Theme {
  if (typeof window === "undefined") return "light";
  const stored = localStorage.getItem("agentcode-theme");
  if (stored === "dark" || stored === "light") return stored;
  return "light";
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<Theme>(getInitialTheme);

  // Load the persisted daemon appearance setting on mount (the real app setting).
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const settings = await daemon.getSettings();
      if (cancelled) return;
      if (settings.appearance === "dark" || settings.appearance === "light") {
        setTheme(settings.appearance);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    localStorage.setItem("agentcode-theme", theme);
    daemon.setSettings({ appearance: theme });
  }, [theme]);

  const toggle = () => setTheme((t) => (t === "light" ? "dark" : "light"));
  const set = (t: Theme) => setTheme(t);

  return (
    <ThemeContext.Provider value={{ theme, toggle, set }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  return useContext(ThemeContext);
}