import { invoke } from '@tauri-apps/api/core';

export interface AdPattern {
  id: number;
  pattern_type: 'keyword' | 'domain' | 'regex';
  pattern_value: string;
  confidence: number;
  source: 'manual' | 'llm_learned';
  hit_count: number;
  consecutive_hits?: number;
  last_hit_at?: string;
  enabled: boolean;
  created_at: string;
}

export interface AdWhitelist {
  id: number;
  url_hash: string;
  source_url: string;
  note?: string;
  created_at: string;
}

export interface AdMarkResult {
  success: boolean;
  patternId?: number;
  llmJudgment?: {
    isAd: boolean;
    confidence: number;
    adKeywords: string[];
    reasoning: string;
  };
  message: string;
}

export interface AdCheckResult {
  isBlocked: boolean;
  matchedPatterns: Array<{
    id: number;
    type: string;
    value: string;
    confidence: number;
  }>;
}

export async function markAsAd(
  itemId: string,
  title: string,
  summary: string,
  sourceUrl: string
): Promise<AdMarkResult> {
  return invoke<AdMarkResult>('mark_as_ad', {
    itemId,
    title,
    summary,
    sourceUrl,
  });
}

export async function listAdPatterns(
  type?: 'keyword' | 'domain' | 'regex',
  enabled?: boolean
): Promise<AdPattern[]> {
  return invoke<AdPattern[]>('list_ad_patterns', { type, enabled });
}

export async function addAdPattern(
  type: 'keyword' | 'domain' | 'regex',
  value: string,
  confidence?: number,
  source: 'manual' | 'llm_learned' = 'manual'
): Promise<AdPattern> {
  return invoke<AdPattern>('add_ad_pattern', {
    type,
    value,
    confidence,
    source,
  });
}

export async function updateAdPattern(
  id: number,
  updates: {
    enabled?: boolean;
    confidence?: number;
  }
): Promise<void> {
  return invoke('update_ad_pattern', { id, ...updates });
}

export async function deleteAdPattern(id: number): Promise<void> {
  return invoke('delete_ad_pattern', { id });
}

export async function checkAdPattern(
  title: string,
  summary: string,
  sourceUrl: string
): Promise<AdCheckResult> {
  return invoke<AdCheckResult>('check_ad_pattern', {
    title,
    summary,
    sourceUrl,
  });
}

export async function addAdWhitelist(
  url: string,
  note?: string
): Promise<void> {
  return invoke('add_ad_whitelist', { url, note });
}

export async function removeAdWhitelist(url: string): Promise<void> {
  return invoke('remove_ad_whitelist', { url });
}

export async function listAdWhitelist(): Promise<AdWhitelist[]> {
  return invoke<AdWhitelist[]>('list_ad_whitelist');
}

export async function getAdStats(): Promise<{
  totalBlocked: number;
  totalWhitelisted: number;
  todayMarked: number;
  hitRate: number;
}> {
  return invoke('get_ad_stats');
}
