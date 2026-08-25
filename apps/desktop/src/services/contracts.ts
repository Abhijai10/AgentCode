import type { ChangeSet, ContextFragment, DaemonStatus, DesktopEvent, Evidence, MemoryFact, Project, Provider, SecurityFinding, SecurityScanner, Session, VerificationCheck } from "../types/domain";
export interface DaemonService { getStatus(): Promise<DaemonStatus>; }
export interface ProjectsService { list(): Promise<Project[]>; }
export interface SessionsService { getCurrent(): Promise<Session>; stream(listener: (event: DesktopEvent) => void): () => void; }
export interface ChangesService { getCurrent(): Promise<ChangeSet>; }
export interface TerminalService { getOutput(): Promise<string[]>; }
export interface VerificationService { list(): Promise<VerificationCheck[]>; }
export interface ContextService { getCurrent(): Promise<ContextFragment[]>; }
export interface MemoryService { list(): Promise<MemoryFact[]>; }
export interface SecurityService { scanners(): Promise<SecurityScanner[]>; findings(): Promise<SecurityFinding[]>; }
export interface ProvidersService { list(): Promise<Provider[]>; }
export interface EvidenceService { list(): Promise<Evidence[]>; }
