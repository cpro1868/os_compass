import { invoke } from '@tauri-apps/api/core';

export interface RadarSource {
  id: number;
  name: string;
  source_type: string;
  url: string;
  platform: string | null;
  enabled: boolean;
  check_interval: number;
  last_checked_at: string | null;
  last_status: string | null;
  last_error: string | null;
}

export interface RadarItem {
  id: number;
  source_id: number;
  project_name: string | null;
  project_url: string | null;
  description: string | null;
  language: string | null;
  status: string;
  published_at: string | null;
  fetched_at: string;
}

export interface ScanResult {
  scanned: number;
  newItems: number;
  errors: number;
}

export async function listRadarSources(): Promise<RadarSource[]> {
  return invoke<RadarSource[]>('list_radar_sources');
}

export async function addRadarSource(
  name: string,
  sourceType: string,
  url: string,
  platform?: string,
  checkInterval?: number
): Promise<RadarSource> {
  return invoke<RadarSource>('add_radar_source', {
    name,
    sourceType,
    url,
    platform,
    checkInterval,
  });
}

export async function updateRadarSource(
  id: number,
  updates: {
    name?: string;
    url?: string;
    enabled?: boolean;
    checkInterval?: number;
  }
): Promise<void> {
  return invoke('update_radar_source', { id, ...updates });
}

export async function deleteRadarSource(id: number): Promise<void> {
  return invoke('delete_radar_source', { id });
}

export async function getRadarItems(
  status?: string,
  limit?: number
): Promise<RadarItem[]> {
  return invoke<RadarItem[]>('get_radar_items', { status, limit });
}

export async function radarItemAction(
  itemId: number,
  action: 'import' | 'blacklist' | 'ignore',
  categoryId?: number
): Promise<{ projectId?: number }> {
  return invoke<{ projectId?: number }>('radar_item_action', {
    itemId,
    action,
    categoryId,
  });
}

export async function triggerRadarScan(
  sourceId?: number
): Promise<ScanResult> {
  return invoke<ScanResult>('trigger_radar_scan', { sourceId });
}

export async function getRadarUnreadCount(): Promise<number> {
  return invoke<number>('get_radar_unread_count');
}

export async function clearRadarCache(beforeDays?: number): Promise<number> {
  return invoke<number>('clear_radar_cache', { beforeDays });
}
