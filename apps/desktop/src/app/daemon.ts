import { invoke } from "@tauri-apps/api/core";
import type {
  DaemonHealth,
  DaemonStatus,
  MissionSummary,
  TaskState,
  ProviderInfo,
  OllamaStatus,
  DiscussSession,
  DiscussMessage,
  DesignSession,
  SettingsState,
  MissionProgress,
  ToolInfo,
  ScannerInfo,
  MemoryInfo,
  SecurityFinding,
  ChangeSet,
} from "./types";

function toDaemonStatus(h: DaemonHealth): DaemonStatus {
  return {
    state: h.lifecycle === "Running" ? "running" : "disconnected",
    instance_id: "",
    uptime: "",
    socket: "",
    database: "",
    recovered_sessions: h.recovered_sessions,
  };
}

export const daemon = {
  async pickProjectFolder(): Promise<string | null> {
    try {
      return await invoke<string | null>("pick_project_folder");
    } catch {
      return null;
    }
  },

  async pickProjectLocation(): Promise<string | null> {
    try {
      return await invoke<string | null>("pick_project_location");
    } catch {
      return null;
    }
  },

  async createProjectFolder(basePath: string, name: string): Promise<string | null> {
    try {
      return await invoke<string>("create_project_folder", { basePath, name });
    } catch {
      return null;
    }
  },

  async health(): Promise<DaemonStatus> {
    try {
      const h = await invoke<DaemonHealth>("daemon_health");
      return toDaemonStatus(h);
    } catch {
      return {
        state: "disconnected",
        instance_id: "",
        uptime: "",
        socket: "",
        database: "",
        recovered_sessions: 0,
      };
    }
  },

  async submitMission(goal: string): Promise<{ mission_id: string; session_id: string } | null> {
    try {
      return await invoke<{ mission_id: string; session_id: string }>("daemon_submit_mission", {
        goal,
      });
    } catch {
      return null;
    }
  },

  async listActiveMissions(): Promise<MissionSummary[]> {
    try {
      return await invoke<MissionSummary[]>("daemon_list_missions");
    } catch {
      return [];
    }
  },

  async getMission(missionId: string): Promise<{ state?: string; tasks: TaskState[] } | null> {
    try {
      return await invoke<{ state?: string; tasks: TaskState[] }>("daemon_get_mission", {
        missionId,
      });
    } catch {
      return null;
    }
  },

  async pauseMission(missionId: string): Promise<boolean> {
    try {
      await invoke("daemon_pause_mission", { missionId });
      return true;
    } catch {
      return false;
    }
  },

  async resumeMission(missionId: string): Promise<boolean> {
    try {
      await invoke("daemon_resume_mission", { missionId });
      return true;
    } catch {
      return false;
    }
  },

  async cancelMission(missionId: string): Promise<boolean> {
    try {
      await invoke("daemon_cancel_mission", { missionId });
      return true;
    } catch {
      return false;
    }
  },

  async listProviders(): Promise<ProviderInfo[]> {
    try {
      return await invoke<ProviderInfo[]>("daemon_list_providers");
    } catch {
      return [];
    }
  },

  async addAccount(
    providerId: string,
    label: string,
    apiKey: string,
    organization?: string,
    project?: string,
    workspace?: string
  ): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_add_account", {
        providerId,
        label,
        apiKey,
        organization: organization || null,
        project: project || null,
        workspace: workspace || null,
      });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async testConnection(
    providerId: string,
    apiKey: string,
    organization?: string,
    project?: string,
    workspace?: string
  ): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_test_connection", {
        providerId,
        apiKey,
        organization: organization || null,
        project: project || null,
        workspace: workspace || null,
      });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async setAccountEnabled(accountId: string, enabled: boolean): Promise<boolean> {
    try {
      await invoke("daemon_set_account_enabled", { accountId, enabled });
      return true;
    } catch {
      return false;
    }
  },

  async rotateAccount(accountId: string, apiKey: string): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_rotate_account", { accountId, apiKey });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async removeAccount(accountId: string): Promise<boolean> {
    try {
      await invoke("daemon_remove_account", { accountId });
      return true;
    } catch {
      return false;
    }
  },

  async discoverOllama(): Promise<OllamaStatus> {
    try {
      return await invoke<OllamaStatus>("daemon_discover_ollama");
    } catch {
      return { running: false, models: [] };
    }
  },

  async refreshOllama(): Promise<OllamaStatus> {
    try {
      return await invoke<OllamaStatus>("daemon_refresh_ollama");
    } catch {
      return { running: false, models: [] };
    }
  },

  async getSettings(): Promise<SettingsState> {
    try {
      return await invoke<SettingsState>("daemon_get_settings");
    } catch {
      return {
        appearance: "light",
        notifications_enabled: true,
        completion_sound: true,
        reduced_motion: false,
        routing_profile: "free_first",
      };
    }
  },

  async setSettings(settings: Partial<SettingsState>): Promise<boolean> {
    try {
      await invoke("daemon_set_settings", { settings });
      return true;
    } catch {
      return false;
    }
  },

  async getMissionProgress(missionId: string): Promise<MissionProgress | null> {
    try {
      return await invoke<MissionProgress>("daemon_mission_progress", { missionId });
    } catch {
      return null;
    }
  },

  async listTools(): Promise<ToolInfo[]> {
    try {
      return await invoke<ToolInfo[]>("daemon_list_tools");
    } catch {
      return [];
    }
  },

  async listScanners(): Promise<ScannerInfo[]> {
    try {
      return await invoke<ScannerInfo[]>("daemon_list_scanners");
    } catch {
      return [];
    }
  },

  async listMemory(): Promise<MemoryInfo[]> {
    try {
      return await invoke<MemoryInfo[]>("daemon_list_memory");
    } catch {
      return [];
    }
  },

  async listSecurityFindings(): Promise<SecurityFinding[]> {
    try {
      return await invoke<SecurityFinding[]>("daemon_security_findings");
    } catch {
      return [];
    }
  },

  async getChangeset(missionId: string): Promise<ChangeSet | null> {
    try {
      return await invoke<ChangeSet>("daemon_get_changeset", { missionId });
    } catch {
      return null;
    }
  },

  async discussListSessions(): Promise<DiscussSession[]> {
    try {
      return await invoke<DiscussSession[]>("daemon_discuss_sessions");
    } catch {
      return [];
    }
  },

  async discussGetMessages(sessionId: string): Promise<DiscussMessage[]> {
    try {
      return await invoke<DiscussMessage[]>("daemon_discuss_messages", { sessionId });
    } catch {
      return [];
    }
  },

  async discussSendMessage(
    sessionId: string,
    content: string
  ): Promise<DiscussMessage | null> {
    try {
      return await invoke<DiscussMessage>("daemon_discuss_send", { sessionId, content });
    } catch {
      return null;
    }
  },

  async designListSessions(): Promise<DesignSession[]> {
    try {
      return await invoke<DesignSession[]>("daemon_design_sessions");
    } catch {
      return [];
    }
  },

  async designGetSession(sessionId: string): Promise<DesignSession | null> {
    try {
      return await invoke<DesignSession>("daemon_design_session", { sessionId });
    } catch {
      return null;
    }
  },
};