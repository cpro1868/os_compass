import { invoke } from '@tauri-apps/api/core';

export interface ProjectMatch {
  project_name: string | null;
  project_url: string | null;
  description: string | null;
  language: string | null;
  source: string;
  match_score: number;
}

export interface SearchResult {
  query: string;
  results: ProjectMatch[];
  total: number;
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

export async function intentSearch(
  query: string,
  conversationId?: string
): Promise<SearchResult> {
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
