import { invoke } from '@tauri-apps/api/core';

export interface PluginInfo {
  id: string;
  name: string;
  plugin_type: string;
  enabled: boolean;
  config: string | null;
  version: string;
  db_mode: string;
  db_path_template: string | null;
}

export async function listFeaturePlugins(): Promise<PluginInfo[]> {
  return invoke<PluginInfo[]>('list_feature_plugins');
}

export async function setPluginEnabled(pluginId: string, enabled: boolean): Promise<void> {
  return invoke('set_plugin_enabled', { pluginId, enabled });
}

export async function getPluginConfig(pluginId: string): Promise<string | null> {
  return invoke<string | null>('get_plugin_config', { pluginId });
}

export async function savePluginConfig(pluginId: string, config: string): Promise<void> {
  return invoke('save_plugin_config', { pluginId, config });
}

export async function pluginGetDbPath(
  pluginId: string,
  vaultDir: string,
  appDataDir: string
): Promise<string> {
  return invoke<string>('plugin_get_db_path', { pluginId, vaultDir, appDataDir });
}
