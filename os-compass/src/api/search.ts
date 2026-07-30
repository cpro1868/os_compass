import { invoke } from '@tauri-apps/api/core';

export interface ProjectMatch {
  name: string;
  url: string;
  description?: string;
  stars?: number;
  forks?: number;
  language?: string;
  health_score?: number;
  match_score: number;
  source: 'local' | 'llm';
}

export interface SearchResult {
  query: string;
  local_results: ProjectMatch[];
  web_results: ProjectMatch[];
  total: number;
  conversation_id: string;
}

export interface SearchHistoryItem {
  id: number;
  query: string;
  result_count: number;
  created_at: string;
}

export interface SearchSource {
  id: number;
  name: string;
  source_type: string;
  url: string;
  platform: string | null;
  enabled: boolean;
}

export interface EmbeddingSettings {
  embedding_enabled: boolean;
  embedding_api_type: string;
  embedding_api_url: string;
  embedding_api_key: string;
  embedding_model: string;
  embedding_dimension: number;
  vss_extension_path: string;
}

export async function intentSearch(query: string, conversationId?: string): Promise<SearchResult> {
  return invoke<SearchResult>('intent_search', { query, conversationId });
}

export async function getSearchHistory(limit?: number): Promise<SearchHistoryItem[]> {
  return invoke<SearchHistoryItem[]>('get_search_history', { limit });
}

export async function clearSearchHistory(): Promise<void> {
  return invoke('clear_search_history');
}

export async function listSearchSources(): Promise<SearchSource[]> {
  return invoke<SearchSource[]>('list_search_sources');
}

export async function addSearchSource(
  name: string,
  sourceType: string,
  url: string,
  platform?: string
): Promise<SearchSource> {
  return invoke<SearchSource>('add_search_source', { name, sourceType, url, platform });
}

export async function updateSearchSource(
  id: number,
  updates: {
    name?: string;
    url?: string;
    enabled?: boolean;
  }
): Promise<void> {
  return invoke('update_search_source', { id, ...updates });
}

export async function deleteSearchSource(id: number): Promise<void> {
  return invoke('delete_search_source', { id });
}

export async function refreshSearchCache(sourceId?: number): Promise<number> {
  return invoke<number>('refresh_search_cache', { sourceId });
}

export async function importSearchResult(
  projectUrl: string,
  projectName: string,
  categoryId?: number
): Promise<{ projectId: number }> {
  return invoke<{ projectId: number }>('import_search_result', {
    projectUrl,
    projectName,
    categoryId,
  });
}

export async function getEmbeddingSettings(): Promise<EmbeddingSettings> {
  return invoke<EmbeddingSettings>('get_embedding_settings');
}

export async function saveEmbeddingSettings(settings: Partial<EmbeddingSettings>): Promise<void> {
  return invoke('save_embedding_settings', { settings });
}

export async function testEmbeddingConnection(): Promise<boolean> {
  return invoke<boolean>('test_embedding_connection');
}

export async function rebuildEmbeddings(projectId?: number): Promise<number> {
  return invoke<number>('rebuild_embeddings', { projectId });
}

export async function generateProjectEmbedding(projectId: number): Promise<number> {
  return invoke<number>('generate_project_embeddings', { projectId });
}
