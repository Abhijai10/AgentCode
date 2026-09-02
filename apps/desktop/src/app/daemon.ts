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
  MissionDetails,
  TaskDetail,
  MissionActivityEvent,
  ChangeSetSummary,
  EvidenceSummaryItem,
  VerificationSummary,
  Conversation,
  ConversationDetail,
  Message,
  Attachment,
  ConversationActivity,
  ProductAnalysis,
  DesignBrief,
  DesignGrammar,
  DesignState,
  DesignCritique,
  DesignRepair,
  DesignPreview,
  DesignPreviewStatus,
  DesignBrowserResult,
  DesignQaReport,
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

  async submitMission(
    goal: string,
    workspaceRoot?: string
  ): Promise<
    | { ok: true; mission_id: string; session_id: string }
    | { ok: false; error: string }
  > {
    try {
      const result = await invoke<{ mission_id: string; session_id: string }>(
        "daemon_submit_mission",
        {
          goal,
          workspaceRoot: workspaceRoot ?? null,
        }
      );
      return { ok: true, mission_id: result.mission_id, session_id: result.session_id };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
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

  async pauseMission(missionId: string): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_pause_mission", { missionId });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async resumeMission(missionId: string): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_resume_mission", { missionId });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async cancelMission(missionId: string): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_cancel_mission", { missionId });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
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

  async getMissionDetails(missionId: string): Promise<MissionDetails | null> {
    try {
      return await invoke<MissionDetails>("daemon_get_mission_details", { missionId });
    } catch {
      return null;
    }
  },

  async getTaskDetails(missionId: string): Promise<TaskDetail[] | null> {
    try {
      const res = await invoke<{ tasks: TaskDetail[] }>("daemon_get_task_details", { missionId });
      return res.tasks ?? null;
    } catch {
      return null;
    }
  },

  async getMissionEvents(missionId: string, limit?: number): Promise<MissionActivityEvent[] | null> {
    try {
      const res = await invoke<{ events: MissionActivityEvent[] }>("daemon_get_mission_events", {
        missionId,
        limit: limit ?? undefined,
      });
      return res.events ?? null;
    } catch {
      return null;
    }
  },

  async getEvidenceSummary(missionId: string): Promise<EvidenceSummaryItem[] | null> {
    try {
      const res = await invoke<{ evidence: EvidenceSummaryItem[] }>("daemon_get_evidence_summary", { missionId });
      return res.evidence ?? null;
    } catch {
      return null;
    }
  },

  async getVerificationSummary(missionId: string): Promise<VerificationSummary | null> {
    try {
      return await invoke<VerificationSummary>("daemon_get_verification_summary", { missionId });
    } catch {
      return null;
    }
  },

  async getChangeSetSummary(missionId: string): Promise<ChangeSetSummary[] | null> {
    try {
      const res = await invoke<{ changesets: ChangeSetSummary[] }>("daemon_get_changeset", { missionId });
      return res.changesets ?? null;
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

  // ── Conversations (real, project-bound chat) ────────────────────────────

  async createConversation(
    projectPath: string,
    mode: string,
    title: string
  ): Promise<{ ok: boolean; conversation_id?: string; error?: string }> {
    try {
      const res = await invoke<{ conversation_id: string }>("daemon_conversation_create", {
        projectPath,
        mode,
        title,
      });
      return { ok: true, conversation_id: res.conversation_id };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async listConversations(projectPath: string): Promise<Conversation[]> {
    try {
      const res = await invoke<{ conversations: Conversation[] }>("daemon_conversation_list", {
        projectPath,
      });
      return res.conversations ?? [];
    } catch {
      return [];
    }
  },

  async getConversation(conversationId: string): Promise<ConversationDetail | null> {
    try {
      const res = await invoke<{ conversation: ConversationDetail }>("daemon_conversation_get", {
        conversationId,
      });
      return res.conversation ?? null;
    } catch {
      return null;
    }
  },

  async renameConversation(
    conversationId: string,
    title: string
  ): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_conversation_rename", { conversationId, title });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async archiveConversation(
    conversationId: string
  ): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_conversation_archive", { conversationId });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async deleteConversation(
    conversationId: string
  ): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_conversation_delete", { conversationId });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async appendMessage(
    conversationId: string,
    role: string,
    content: string,
    missionRef?: string
  ): Promise<{ ok: boolean; message?: Message; error?: string }> {
    try {
      const res = await invoke<{ message: Message }>("daemon_message_append", {
        conversationId,
        role,
        content,
        missionRef: missionRef ?? null,
      });
      return { ok: true, message: res.message };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async submitGoalFromConversation(
    conversationId: string,
    goal: string,
    attachmentIds?: string[]
  ): Promise<
    | { ok: true; mission_id: string; session_id: string }
    | { ok: false; error: string }
  > {
    try {
      const result = await invoke<{ mission_id: string; session_id: string }>(
        "daemon_goal_submit",
        {
          conversationId,
          goal,
          attachmentIds: attachmentIds ?? [],
        }
      );
      return { ok: true, mission_id: result.mission_id, session_id: result.session_id };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async discussSend(
    conversationId: string,
    content: string,
    attachmentIds?: string[]
  ): Promise<{ ok: boolean; message?: Message; error?: string }> {
    try {
      const res = await invoke<{ message: Message }>("daemon_discuss_send", {
        conversationId,
        content,
        attachmentIds: attachmentIds ?? [],
      });
      return { ok: true, message: res.message };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async addAttachment(
    conversationId: string,
    projectPath: string
  ): Promise<{ ok: boolean; attachment?: Attachment; error?: string }> {
    try {
      const res = await invoke<{ attachment: Attachment }>("daemon_add_attachment", {
        conversationId,
        projectPath,
      });
      return { ok: true, attachment: res.attachment };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async attachmentPath(
    attachmentId: string,
    projectPath: string
  ): Promise<string | null> {
    try {
      const res = await invoke<{ path: string }>("daemon_attachment_path", {
        attachmentId,
        projectPath,
      });
      return res.path ?? null;
    } catch {
      return null;
    }
  },

  async listAttachments(conversationId: string): Promise<Attachment[]> {
    try {
      const res = await invoke<{ attachments: Attachment[] }>("daemon_attachment_list", {
        conversationId,
      });
      return res.attachments ?? [];
    } catch {
      return [];
    }
  },

  async removeAttachment(
    attachmentId: string
  ): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_attachment_remove", { attachmentId });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async getConversationActivity(conversationId: string): Promise<ConversationActivity | null> {
    try {
      return await invoke<ConversationActivity>("daemon_conversation_activity", {
        conversationId,
      });
    } catch {
      return null;
    }
  },

  // ── Design Studio (G4) ────────────────────────────────────────────────

  async designSend(
    conversationId: string,
    content: string,
    attachmentIds?: string[]
  ): Promise<{ ok: boolean; message?: Message; error?: string }> {
    try {
      const res = await invoke<{ message: Message }>("daemon_design_send", {
        conversationId,
        content,
        attachmentIds: attachmentIds ?? [],
      });
      return { ok: true, message: res.message };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designUnderstand(
    conversationId: string
  ): Promise<{ ok: boolean; analysis?: ProductAnalysis; error?: string }> {
    try {
      const res = await invoke<{ analysis: ProductAnalysis }>(
        "daemon_design_understand",
        { conversationId }
      );
      return { ok: true, analysis: res.analysis };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designBrief(
    conversationId: string,
    audience: string,
    workflow: string
  ): Promise<{ ok: boolean; brief?: DesignBrief; error?: string }> {
    try {
      const res = await invoke<{ brief: DesignBrief }>("daemon_design_brief", {
        conversationId,
        audience,
        workflow,
      });
      return { ok: true, brief: res.brief };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designGrammar(
    conversationId: string
  ): Promise<{ ok: boolean; grammar?: DesignGrammar; error?: string }> {
    try {
      const res = await invoke<{ grammar: DesignGrammar }>("daemon_design_grammar", {
        conversationId,
      });
      return { ok: true, grammar: res.grammar };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designState(
    conversationId: string
  ): Promise<{ ok: boolean; state?: DesignState; error?: string }> {
    try {
      const res = await invoke<{ design_state: DesignState }>("daemon_design_state", {
        conversationId,
      });
      return { ok: true, state: res.design_state };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designCritique(
    conversationId: string,
    content: string,
    docType?: string
  ): Promise<{ ok: boolean; critique?: DesignCritique; error?: string }> {
    try {
      const res = await invoke<{ critique: DesignCritique }>("daemon_design_critique", {
        conversationId,
        content,
        docType: docType ?? "implementation",
      });
      return { ok: true, critique: res.critique };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designRepair(
    conversationId: string,
    content: string,
    docType?: string
  ): Promise<{ ok: boolean; repair?: DesignRepair; error?: string }> {
    try {
      const res = await invoke<{ repair: DesignRepair }>("daemon_design_repair", {
        conversationId,
        content,
        docType: docType ?? "implementation",
      });
      return { ok: true, repair: res.repair };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designPreviewStart(
    conversationId: string
  ): Promise<{ ok: boolean; preview?: DesignPreview; error?: string }> {
    try {
      const res = await invoke<{ preview: DesignPreview }>("daemon_design_preview_start", {
        conversationId,
      });
      return { ok: true, preview: res.preview };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designPreviewStatus(
    conversationId: string
  ): Promise<{ ok: boolean; preview?: DesignPreviewStatus; error?: string }> {
    try {
      const res = await invoke<{ preview: DesignPreviewStatus }>(
        "daemon_design_preview_status",
        { conversationId }
      );
      return { ok: true, preview: res.preview };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designPreviewStop(
    conversationId: string
  ): Promise<{ ok: boolean; error?: string }> {
    try {
      await invoke("daemon_design_preview_stop", { conversationId });
      return { ok: true };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designBrowser(
    conversationId: string,
    url?: string,
    html?: string,
    deterministic?: boolean,
    viewportHint?: string
  ): Promise<{ ok: boolean; browser?: DesignBrowserResult; error?: string }> {
    try {
      const res = await invoke<{ browser: DesignBrowserResult }>("daemon_design_browser", {
        conversationId,
        url: url ?? "",
        html: html ?? "",
        deterministic: deterministic ?? false,
        viewportHint: viewportHint ?? "desktop",
      });
      return { ok: true, browser: res.browser };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designQa(
    conversationId: string,
    kind: "responsive" | "accessibility" | "functional",
    content: string
  ): Promise<{ ok: boolean; qa?: DesignQaReport; error?: string }> {
    try {
      const command =
        kind === "responsive"
          ? "daemon_design_qa_responsive"
          : kind === "accessibility"
            ? "daemon_design_qa_accessibility"
            : "daemon_design_qa_functional";
      const res = await invoke<{ qa: DesignQaReport }>(command, {
        conversationId,
        content,
      });
      return { ok: true, qa: res.qa };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },
};