import { invoke } from '@tauri-apps/api/core';

export interface ExploreSource {
  type: 'readme_chunk' | 'ai_report' | 'runbook' | 'notes';
  chunkIndex?: number | null;
}

export interface ExploreAnswer {
  sessionId: number;
  answer: string;
  sources: ExploreSource[];
  degraded: boolean;
}

export interface ExploreSession {
  id: number;
  projectId: number;
  title: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ExploreMessage {
  id: number;
  sessionId: number;
  role: 'user' | 'assistant';
  content: string;
  sources: string | null;   // JSON 字符串，解析后为 ExploreSource[]
  createdAt: string;
}

export interface ExploreKbStatus {
  ready: boolean;
  building: boolean;
  degraded: boolean;
  readmeChunks: number;
  hasAiReport: boolean;
  hasRunbook: boolean;
  notesCount: number;
}

export async function askExplore(projectId: number, sessionId: number | undefined, question: string): Promise<ExploreAnswer> {
  return invoke<ExploreAnswer>('ask_explore', { projectId, sessionId, question });
}
export async function listExploreSessions(projectId: number): Promise<ExploreSession[]> {
  return invoke<ExploreSession[]>('list_explore_sessions', { projectId });
}
export async function getExploreMessages(sessionId: number): Promise<ExploreMessage[]> {
  return invoke<ExploreMessage[]>('get_explore_messages', { sessionId });
}
export async function deleteExploreSession(sessionId: number): Promise<void> {
  return invoke<void>('delete_explore_session', { sessionId });
}
export async function getLatestExploreSession(projectId: number): Promise<ExploreSession | null> {
  return invoke<ExploreSession | null>('get_latest_explore_session', { projectId });
}
export async function getExploreKbStatus(projectId: number): Promise<ExploreKbStatus> {
  return invoke<ExploreKbStatus>('get_explore_kb_status', { projectId });
}
