import { invoke } from "@tauri-apps/api/core";
import type { SystemVariable, SourcePlugin } from "../types";

export async function getSystemVariables(): Promise<SystemVariable[]> {
  return invoke("get_system_variables");
}

export async function getSystemVariable(key: string): Promise<SystemVariable | null> {
  return invoke("get_system_variable", { key });
}

export async function setSystemVariable(
  key: string,
  value: string,
  isSecret: boolean
): Promise<void> {
  return invoke("set_system_variable", { key, value, isSecret });
}

export async function deleteSystemVariable(key: string): Promise<void> {
  return invoke("delete_system_variable", { key });
}

export async function getVariable(key: string): Promise<string | null> {
  return invoke("get_variable", { key });
}

export async function listExtensions(): Promise<SourcePlugin[]> {
  return invoke("list_extensions");
}

export async function getExtension(id: string): Promise<SourcePlugin | null> {
  return invoke("get_extension", { id });
}

export async function setExtensionEnabled(
  id: string,
  enabled: boolean
): Promise<void> {
  return invoke("set_extension_enabled", { id, enabled });
}

export async function getEnabledExtensions(): Promise<SourcePlugin[]> {
  return invoke("get_enabled_extensions");
}
