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
  DiscussPlan,
  DiscussDecisionRecord,
  DesignSession,
  SettingsState,
  MissionProgress,
  ToolInfo,
  ScannerInfo,
  MemoryInfo,
  SecurityFinding,
  SecurityScope,
  SecurityAuditResult,
  SecurityModeFinding,
  SecurityValidation,
  SecurityAttackPath,
  SecuritySuppression,
  SecurityRiskAcceptance,
  SecurityReportData,
  SecurityStatus,
  SecuritySecretLifecycle,
  SecurityQualityMetrics,
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
  ReferenceAnalysis,
  DesignBrief,
  DesignGrammar,
  DesignState,
  DesignCritique,
  DesignRepair,
  DesignPreview,
  DesignPreviewStatus,
  DesignBrowserResult,
  DesignQaReport,
  VisualCritique,
  DesignConstraint,
  DesignConstraints,
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

  async providerPreferencesGet(): Promise<{
    ok: boolean;
    preferences?: {
      routing_profile: string;
      preferred_model: string;
      updated_at_ms: number;
      configured: boolean;
    };
    error?: string;
  }> {
    try {
      const res = await invoke<{ preferences: unknown }>(
        "daemon_provider_preferences_get"
      );
      return {
        ok: true,
        preferences: res.preferences as {
          routing_profile: string;
          preferred_model: string;
          updated_at_ms: number;
          configured: boolean;
        },
      };
    } catch (error) {
      return { ok: false, error: String(error) };
    }
  },

  async providerPreferencesSet(
    routingProfile: string,
    preferredModel: string
  ): Promise<{ ok: boolean; preferences?: unknown; error?: string }> {
    try {
      const res = await invoke<{ preferences: unknown }>(
        "daemon_provider_preferences_set",
        { routingProfile, preferredModel }
      );
      return { ok: true, preferences: res.preferences };
    } catch (error) {
      return { ok: false, error: String(error) };
    }
  },

  async providerHealthGet(): Promise<{
    ok: boolean;
    health?: {
      observations: {
        account_id: string;
        provider_id: string;
        success: boolean;
        latency_ms: number;
        failure_code: string;
        failure_message: string;
        observed_at_ms: number;
      }[];
      catalog: { id: string; display_name: string; pricing_classification: string }[];
    };
    error?: string;
  }> {
    try {
      const res = await invoke<{ health: unknown }>("daemon_provider_health_get");
      return {
        ok: true,
        health: res.health as {
          observations: {
            account_id: string;
            provider_id: string;
            success: boolean;
            latency_ms: number;
            failure_code: string;
            failure_message: string;
            observed_at_ms: number;
          }[];
          catalog: { id: string; display_name: string; pricing_classification: string }[];
        },
      };
    } catch (error) {
      return { ok: false, error: String(error) };
    }
  },

  async readinessGet(): Promise<{
    ok: boolean;
    readiness?: {
      daemon_lifecycle: string;
      provider_accounts_configured: number;
      providers_with_accounts: string[];
      ollama: { running: boolean; model_count: number; models: { id: string; model_name: string }[] };
      scanner_note: string;
      usable_route: boolean;
    };
    error?: string;
  }> {
    try {
      const res = await invoke<{ readiness: unknown }>("daemon_readiness_get");
      return {
        ok: true,
        readiness: res.readiness as {
          daemon_lifecycle: string;
          provider_accounts_configured: number;
          providers_with_accounts: string[];
          ollama: { running: boolean; model_count: number; models: { id: string; model_name: string }[] };
          scanner_note: string;
          usable_route: boolean;
        },
      };
    } catch (error) {
      return { ok: false, error: String(error) };
    }
  },

  async missionExport(
    missionId: string
  ): Promise<{ ok: boolean; export?: unknown; error?: string }> {
    try {
      const res = await invoke<{ export: unknown }>("daemon_mission_export", {
        missionId,
      });
      return { ok: true, export: res.export };
    } catch (error) {
      return { ok: false, error: String(error) };
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

  async discussTurnIntoPlan(
    conversationId: string
  ): Promise<{ ok: boolean; plan?: DiscussPlan; error?: string }> {
    try {
      const res = await invoke<{ plan: DiscussPlan }>(
        "daemon_discuss_turn_into_plan",
        { conversationId }
      );
      return { ok: true, plan: res.plan };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async discussExecutePlan(
    conversationId: string
  ): Promise<{ ok: boolean; missionId?: string; planGoal?: string; error?: string }> {
    try {
      const res = await invoke<{ mission_id: string; plan_goal: string }>(
        "daemon_discuss_execute_plan",
        { conversationId }
      );
      return { ok: true, missionId: res.mission_id, planGoal: res.plan_goal };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async discussAcceptDecision(
    conversationId: string,
    messageId: string,
    decision: string,
    rationale?: string
  ): Promise<{ ok: boolean; decisionRecord?: DiscussDecisionRecord; error?: string }> {
    try {
      const res = await invoke<{ decision_record: DiscussDecisionRecord }>(
        "daemon_discuss_accept_decision",
        { conversationId, messageId, decision, rationale: rationale ?? "" }
      );
      return { ok: true, decisionRecord: res.decision_record };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async discussGetPlan(
    conversationId: string
  ): Promise<{ ok: boolean; plan?: string | null; error?: string }> {
    try {
      const res = await invoke<{ plan: string | null }>("daemon_discuss_plan_get", {
        conversationId,
      });
      return { ok: true, plan: res.plan };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async discussGetDecisions(
    conversationId: string
  ): Promise<{ ok: boolean; decisions?: string; error?: string }> {
    try {
      const res = await invoke<{ decisions: string }>("daemon_discuss_decisions_get", {
        conversationId,
      });
      return { ok: true, decisions: res.decisions };
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

  async designAnalyzeReference(
    conversationId: string,
    attachmentId: string
  ): Promise<{ ok: boolean; analysis?: ReferenceAnalysis; error?: string }> {
    try {
      const res = await invoke<{ analysis: ReferenceAnalysis }>(
        "daemon_design_analyze_reference",
        { conversationId, attachmentId }
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
    content?: string,
    url?: string,
    html?: string,
    deterministic?: boolean,
    viewportHint?: string
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
        content: content ?? "",
        url: url ?? "",
        html: html ?? "",
        deterministic: deterministic ?? false,
        viewportHint: viewportHint ?? "desktop",
      });
      return { ok: true, qa: res.qa };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designQaReport(
    conversationId: string,
    url?: string,
    html?: string,
    deterministic?: boolean,
    viewportHint?: string
  ): Promise<{ ok: boolean; qa?: DesignQaReport; error?: string }> {
    try {
      const res = await invoke<{ qa: DesignQaReport }>("daemon_design_qa_report", {
        conversationId,
        url: url ?? "",
        html: html ?? "",
        deterministic: deterministic ?? false,
        viewportHint: viewportHint ?? "desktop",
      });
      return { ok: true, qa: res.qa };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designVisualCritique(
    conversationId: string,
    url?: string,
    deterministic?: boolean
  ): Promise<{ ok: boolean; visualCritique?: VisualCritique; error?: string }> {
    try {
      const res = await invoke<{ visual_critique: VisualCritique }>(
        "daemon_design_visual_critique",
        { conversationId, url: url ?? "", deterministic: deterministic ?? false }
      );
      return { ok: true, visualCritique: res.visual_critique };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designExecuteContract(
    conversationId: string
  ): Promise<{ ok: boolean; missionId?: string; contractGoal?: string; error?: string }> {
    try {
      const res = await invoke<{ mission_id: string; contract_goal: string }>(
        "daemon_design_execute_contract",
        { conversationId }
      );
      return { ok: true, missionId: res.mission_id, contractGoal: res.contract_goal };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designConstraintsGet(
    conversationId: string
  ): Promise<{ ok: boolean; constraints?: DesignConstraints; error?: string }> {
    try {
      const res = await invoke<{ constraints: DesignConstraints }>(
        "daemon_design_constraints_get",
        { conversationId }
      );
      return { ok: true, constraints: res.constraints };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designConstraintsSet(
    conversationId: string,
    constraints: DesignConstraint[]
  ): Promise<{ ok: boolean; constraints?: DesignConstraints; error?: string }> {
    try {
      const res = await invoke<{ constraints: DesignConstraints }>(
        "daemon_design_constraints_set",
        { conversationId, constraints }
      );
      return { ok: true, constraints: res.constraints };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async designMemoryGet(
    conversationId: string
  ): Promise<{ ok: boolean; memory?: unknown; error?: string }> {
    try {
      const res = await invoke<{ memory: unknown }>(
        "daemon_design_memory_get",
        { conversationId }
      );
      return { ok: true, memory: res.memory };
    } catch (error) {
      return { ok: false, error: String(error) };
    }
  },

  async designIterations(
    conversationId: string
  ): Promise<{ ok: boolean; iterations?: unknown; error?: string }> {
    try {
      const res = await invoke<{ iterations: unknown }>(
        "daemon_design_iterations",
        { conversationId }
      );
      return { ok: true, iterations: res.iterations };
    } catch (error) {
      return { ok: false, error: String(error) };
    }
  },

  async designMaterializeState(
    conversationId: string
  ): Promise<{ ok: boolean; materialization?: unknown; error?: string }> {
    try {
      const res = await invoke<{ materialization: unknown }>(
        "daemon_design_materialize_state",
        { conversationId }
      );
      return { ok: true, materialization: res.materialization };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  // ── Security Mode (G5) ──────────────────────────────────────────────────

  async securitySend(
    conversationId: string,
    content: string,
    attachmentIds?: string[]
  ): Promise<{ ok: boolean; message?: Message; error?: string }> {
    try {
      const res = await invoke<{ message: Message }>("daemon_security_send", {
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

  async securitySetScope(
    conversationId: string,
    target: string,
    scopeKind: string,
    authState: string,
    allowedHosts?: string[],
    allowedPorts?: number[],
    allowedTechniques?: string[]
  ): Promise<{ ok: boolean; scope?: SecurityScope; error?: string }> {
    try {
      const res = await invoke<{ scope: SecurityScope }>(
        "daemon_security_scope_set",
        {
          conversationId,
          target,
          scopeKind,
          authState,
          allowedHosts: allowedHosts ?? [],
          allowedPorts: allowedPorts ?? [],
          allowedTechniques: allowedTechniques ?? [],
        }
      );
      return { ok: true, scope: res.scope };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityAudit(
    conversationId: string,
    depth?: "quick" | "full" | "cloud" | "ai" | "adversarial"
  ): Promise<{ ok: boolean; audit?: SecurityAuditResult; error?: string }> {
    try {
      const res = await invoke<{ audit: SecurityAuditResult }>(
        "daemon_security_audit",
        { conversationId, depth: depth ?? "quick" }
      );
      return { ok: true, audit: res.audit };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityFindings(
    conversationId: string
  ): Promise<{ ok: boolean; findings?: SecurityModeFinding[]; error?: string }> {
    try {
      const res = await invoke<{ result: { findings: SecurityModeFinding[] } }>(
        "daemon_security_findings",
        { conversationId }
      );
      return { ok: true, findings: res.result?.findings ?? [] };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityFindingDetail(
    conversationId: string,
    findingId: string
  ): Promise<{ ok: boolean; finding?: SecurityModeFinding; error?: string }> {
    try {
      const res = await invoke<{ finding: SecurityModeFinding }>(
        "daemon_security_finding_detail",
        { conversationId, findingId }
      );
      return { ok: true, finding: res.finding };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityFindingTransition(
    conversationId: string,
    findingId: string,
    targetState: string
  ): Promise<{ ok: boolean; finding?: SecurityModeFinding; error?: string }> {
    try {
      const res = await invoke<{ finding: SecurityModeFinding }>(
        "daemon_security_finding_transition",
        { conversationId, findingId, targetState }
      );
      return { ok: true, finding: res.finding };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityValidate(
    conversationId: string,
    findingId: string,
    canary?: string
  ): Promise<{ ok: boolean; validation?: SecurityValidation; error?: string }> {
    try {
      const res = await invoke<{ validation: SecurityValidation }>(
        "daemon_security_validate",
        { conversationId, findingId, canary: canary ?? null }
      );
      return { ok: true, validation: res.validation };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityAttackPaths(
    conversationId: string
  ): Promise<{ ok: boolean; attackPaths?: SecurityAttackPath[]; error?: string }> {
    try {
      const res = await invoke<{ result: { attack_paths: SecurityAttackPath[] } }>(
        "daemon_security_attack_paths",
        { conversationId }
      );
      return { ok: true, attackPaths: res.result?.attack_paths ?? [] };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityRemediate(
    conversationId: string,
    findingId: string,
    approved: boolean
  ): Promise<{ ok: boolean; remediation?: { mission_id: string; finding_state: string; conversation_id: string }; error?: string }> {
    try {
      const res = await invoke<{ remediation: { mission_id: string; finding_state: string; conversation_id: string } }>(
        "daemon_security_remediate",
        { conversationId, findingId, approved }
      );
      return { ok: true, remediation: res.remediation };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityRetest(
    conversationId: string
  ): Promise<{ ok: boolean; retest?: { closed: string[]; reopened: string[]; retesting: string[]; source_commit: string }; error?: string }> {
    try {
      const res = await invoke<{ retest: { closed: string[]; reopened: string[]; retesting: string[]; source_commit: string } }>(
        "daemon_security_retest",
        { conversationId }
      );
      return { ok: true, retest: res.retest };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securitySuppress(
    conversationId: string,
    findingId: string,
    reason: string,
    expiresAtMs?: number,
    applicability?: string,
    compensatingControls?: string
  ): Promise<{ ok: boolean; suppression?: SecuritySuppression; error?: string }> {
    try {
      const res = await invoke<{ suppression: SecuritySuppression }>(
        "daemon_security_suppress",
        {
          conversationId,
          findingId,
          reason,
          expiresAtMs: expiresAtMs ?? null,
          applicability: applicability ?? "",
          compensatingControls: compensatingControls ?? "",
        }
      );
      return { ok: true, suppression: res.suppression };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityAcceptRisk(
    conversationId: string,
    findingId: string,
    rationale: string,
    approver: string,
    expiresAtMs?: number
  ): Promise<{ ok: boolean; riskAcceptance?: SecurityRiskAcceptance; error?: string }> {
    try {
      const res = await invoke<{ risk_acceptance: SecurityRiskAcceptance }>(
        "daemon_security_accept_risk",
        {
          conversationId,
          findingId,
          rationale,
          approver,
          expiresAtMs: expiresAtMs ?? null,
        }
      );
      return { ok: true, riskAcceptance: res.risk_acceptance };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityReport(
    conversationId: string
  ): Promise<{ ok: boolean; report?: SecurityReportData; error?: string }> {
    try {
      const res = await invoke<{ report: SecurityReportData }>(
        "daemon_security_report",
        { conversationId }
      );
      return { ok: true, report: res.report };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityStatus(
    conversationId: string
  ): Promise<{ ok: boolean; status?: SecurityStatus; error?: string }> {
    try {
      const res = await invoke<{ status: SecurityStatus }>(
        "daemon_security_status",
        { conversationId }
      );
      return { ok: true, status: res.status };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securitySecretLifecycle(
    conversationId: string,
    findingId: string
  ): Promise<{ ok: boolean; lifecycle?: SecuritySecretLifecycle; error?: string }> {
    try {
      const res = await invoke<{ lifecycle: SecuritySecretLifecycle }>(
        "daemon_security_secret_lifecycle",
        { conversationId, findingId }
      );
      return { ok: true, lifecycle: res.lifecycle };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },

  async securityQualityMetrics(
    conversationId: string
  ): Promise<{ ok: boolean; metrics?: SecurityQualityMetrics; error?: string }> {
    try {
      const res = await invoke<{ metrics: SecurityQualityMetrics }>(
        "daemon_security_quality_metrics",
        { conversationId }
      );
      return { ok: true, metrics: res.metrics };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : String(e);
      const cleaned = message.replace(/^[A-Z0-9_-]+:\s*/, "");
      return { ok: false, error: cleaned || message };
    }
  },
};