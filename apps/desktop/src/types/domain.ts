export type SessionMode = "agent" | "discuss" | "design";
export type MissionState = "working" | "waiting" | "verifying" | "blocked" | "complete" | "recovering";
export type TaskState = "done" | "in_progress" | "pending" | "blocked" | "verifying";
export interface Project { id: string; name: string; color: string; }
export interface Session { id: string; title: string; projectId: string; mode: SessionMode; branch: string; }
export interface Mission { id: string; sessionId: string; state: MissionState; summary: string; }
export interface Task { id: string; title: string; state: TaskState; detail?: string; }
export interface AgentEvent { id: string; kind: "message" | "decision" | "activity"; content: string; timestamp: string; }
export interface ToolCall { id: string; label: string; path: string; lines: string; code: string; status: "complete" | "running"; }
export interface ChangedFile { path: string; additions: number; deletions: number; diff: string; }
export interface ChangeSet { id: string; files: ChangedFile[]; summary: string[]; safe: boolean; }
export interface VerificationCheck { name: string; state: "passed" | "failed" | "blocked" | "unavailable" | "stale"; detail: string; }
export interface ContextFragment { label: string; value: string; }
export interface MemoryFact { id: string; statement: string; type: string; freshness: string; source: string; date: string; }
export interface SecurityScanner { name: string; state: "available" | "unavailable" | "running" | "passed" | "findings"; }
export interface SecurityFinding { id: string; scanner: string; title: string; severity: "info" | "low" | "medium" | "high"; heuristic: boolean; }
export interface Evidence { id: string; category: "Logs" | "Artifacts" | "Screenshots" | "Reports"; label: string; count: number; }
export interface DaemonStatus { state: "running" | "disconnected" | "recovering"; instanceId: string; uptime: string; socket: string; database: string; }
export interface Provider { id: string; name: string; state: "connected" | "disconnected" | "unavailable"; }
export interface Model { id: string; name: string; providerId: string; }
export type DesktopEvent = { type: "MissionStateChanged"; mission: Mission } | { type: "TaskStateChanged"; task: Task } | { type: "AgentMessage"; event: AgentEvent } | { type: "ToolStarted" | "ToolCompleted"; tool: ToolCall } | { type: "ChangeSetUpdated"; changeSet: ChangeSet } | { type: "VerificationUpdated"; checks: VerificationCheck[] } | { type: "ApprovalRequired"; missionId: string } | { type: "DaemonStateChanged"; daemon: DaemonStatus };
