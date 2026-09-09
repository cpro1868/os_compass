import { invoke } from '@tauri-apps/api/core';

export interface RadarSource {
  id: number;
  name: string;
  source_type: string;
  url: string;
  platform: string | null;
  enabled: boolean;
  check_interval: number;
  proxy_enabled: boolean;
  proxy_protocol: string;
  proxy_host: string | null;
  proxy_port: number;
  proxy_username: string | null;
  proxy_password: string | null;
  last_checked_at: string | null;
  last_status: string | null;
  last_error: string | null;
}

export interface RadarItem {
  id: number;
  source_id: number;
  source_name: string | null;
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

export interface RadarSourceInput {
  name: string;
  sourceType: string;
  url: string;
  platform?: string;
  checkInterval?: number;
  proxyEnabled?: boolean;
  proxyProtocol?: string;
  proxyHost?: string;
  proxyPort?: number;
  proxyUsername?: string;
  proxyPassword?: string;
}

export interface RadarSourceUpdate {
  name?: string;
  url?: string;
  enabled?: boolean;
  checkInterval?: number;
  proxyEnabled?: boolean;
  proxyProtocol?: string;
  proxyHost?: string;
  proxyPort?: number;
  proxyUsername?: string;
  proxyPassword?: string;
}

export async function listRadarSources(): Promise<RadarSource[]> {
  return invoke<RadarSource[]>('list_radar_sources');
}

export async function addRadarSource(input: RadarSourceInput): Promise<RadarSource> {
  return invoke<RadarSource>('add_radar_source', {
    name: input.name,
    sourceType: input.sourceType,
    url: input.url,
    platform: input.platform,
    checkInterval: input.checkInterval,
    proxyEnabled: input.proxyEnabled,
    proxyProtocol: input.proxyProtocol,
    proxyHost: input.proxyHost,
    proxyPort: input.proxyPort,
    proxyUsername: input.proxyUsername,
    proxyPassword: input.proxyPassword,
  });
}

export async function updateRadarSource(
  id: number,
  updates: RadarSourceUpdate
): Promise<void> {
  return invoke('update_radar_source', { id, ...updates });
}

export async function deleteRadarSource(id: number): Promise<void> {
  return invoke('delete_radar_source', { id });
}

export interface RadarItemsResult {
  items: RadarItem[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export async function getRadarItems(
  status?: string,
  limit?: number,
  keyword?: string,
  sourceIds?: string,
  startDate?: string,
  endDate?: string,
  page?: number,
  pageSize?: number
): Promise<RadarItemsResult> {
  return invoke<RadarItemsResult>('get_radar_items', {
    status,
    limit,
    keyword,
    source_ids: sourceIds,
    start_date: startDate,
    end_date: endDate,
    page,
    page_size: pageSize,
  });
}

export async function radarItemAction(
  itemId: number,
  action: 'collect' | 'blacklist' | 'delete',
  categoryId?: number
): Promise<null> {
  return invoke('radar_item_action', {
    itemId,
    action,
    categoryId,
  });
}

export async function triggerRadarScan(
  sourceId?: number,
  timeRange?: string
): Promise<ScanResult> {
  return invoke<ScanResult>('trigger_radar_scan', { sourceId, timeRange });
}

export async function getRadarUnreadCount(): Promise<number> {
  return invoke<number>('get_radar_unread_count');
}

export async function clearRadarCache(beforeDays?: number): Promise<number> {
  return invoke<number>('clear_radar_cache', { beforeDays });
}

export async function clearRadarAll(): Promise<number> {
  return invoke<number>('clear_radar_all');
}

export async function getSupportedPlatformDomains(): Promise<string[]> {
  return invoke<string[]>('get_supported_platform_domains');
}

export interface RadarSchedule {
  enabled: boolean;
  mode: 'interval' | 'custom';
  interval_seconds: number;
  custom_unit: 'second' | 'minute' | 'hour' | null;
  custom_value: number | null;
  notification_enabled: boolean;
  system_notification: boolean;
  badge_notification: boolean;
  last_run_at: string | null;
  next_run_at: string | null;
  new_items_count: number;
}

export interface RadarScheduleUpdate {
  enabled?: boolean;
  mode?: 'interval' | 'custom';
  interval_seconds?: number;
  custom_unit?: 'second' | 'minute' | 'hour';
  custom_value?: number;
  notification_enabled?: boolean;
  system_notification?: boolean;
  badge_notification?: boolean;
}

export interface Notification {
  id: number;
  title: string;
  body: string | null;
  item_count: number;
  read: boolean;
  created_at: string;
}

export async function getRadarSchedule(): Promise<RadarSchedule> {
  return invoke<RadarSchedule>('get_radar_schedule');
}

export async function updateRadarSchedule(update: RadarScheduleUpdate): Promise<void> {
  return invoke('update_radar_schedule', { update });
}

export async function triggerRadarScanNow(): Promise<ScanResult> {
  return invoke<ScanResult>('trigger_radar_scan_now');
}

export async function listRadarNotifications(unreadOnly?: boolean): Promise<Notification[]> {
  return invoke<Notification[]>('list_radar_notifications', { unreadOnly });
}

export async function markRadarNotificationRead(id: number): Promise<void> {
  return invoke('mark_radar_notification_read', { id });
}

export async function clearRadarNotifications(): Promise<void> {
  return invoke('clear_radar_notifications');
}

export async function getRadarUnreadNotificationCount(): Promise<number> {
  return invoke<number>('get_radar_unread_notification_count');
}
