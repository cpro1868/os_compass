import { invoke } from "@tauri-apps/api/core";
import type { Project, Category, Tag, ProjectNote } from "../types";
export { tagApi, type Tag, type CreateTagInput } from "./tag";

export * from "./extension";
export * from "./plugin";
export * from "./radar";
export * from "./search";

export interface ImportInput {
  url: string;
  category_id?: number;
  generate_ai_report?: boolean;
  auto_translate?: boolean;
}

export interface ImportResponse {
  success: boolean;
  project_id: number | null;
  error: string | null;
  data: {
    name: string;
    url: string;
    source: string;
    description: string | null;
    stars: number;
    readme_content?: string | null;
  } | null;
  duplicate?: {
    project_id: number;
    project_name: string;
    url: string;
  } | null;
}

export interface ProjectData {
  name: string;
  url: string;
  source: string;
  source_id: string;
  description: string | null;
  languages: string | null;
  stars: number;
  forks: number;
  open_issues: number;
  license: string | null;
  homepage: string | null;
  latest_commit: string | null;
  latest_release: string | null;
  readme_content: string | null;
}

export interface ImportPreview {
  success: boolean;
  error: string | null;
  project_data: ProjectData | null;
}

export async function getProjects(): Promise<Project[]> {
  return invoke("get_projects");
}

export async function getProject(id: number): Promise<Project | null> {
  return invoke("get_project", { id });
}

export async function createProject(input: Partial<Project>): Promise<number> {
  return invoke("create_project", { input });
}

export async function updateProject(input: Partial<Project>): Promise<void> {
  return invoke("update_project", { input });
}

export async function updateProjectStatus(id: number, lifecycleStatus: string): Promise<void> {
  return invoke("update_project_status", { id, lifecycleStatus });
}

export async function deleteProject(id: number, permanent: boolean = false): Promise<void> {
  return invoke("delete_project", { id, permanent });
}

export async function archiveProject(id: number): Promise<void> {
  return invoke("archive_project", { id });
}

export async function restoreProject(id: number): Promise<void> {
  return invoke("restore_project", { id });
}

export async function getArchivedProjects(): Promise<Project[]> {
  return invoke("get_archived_projects");
}

export async function importProject(input: ImportInput): Promise<ImportResponse> {
  return invoke("import_project", { input });
}

export async function previewImport(url: string): Promise<ImportPreview> {
  return invoke("preview_import", { url });
}

export async function getCategories(): Promise<Category[]> {
  return invoke("get_categories");
}

export async function createCategory(input: Partial<Category>): Promise<number> {
  return invoke("create_category", { input });
}

export async function updateCategory(input: Category): Promise<void> {
  return invoke("update_category", { input });
}

export async function getTags(): Promise<Tag[]> {
  return invoke("get_tags");
}

export async function createTag(input: Partial<Tag>): Promise<number> {
  return invoke("create_tag", { input });
}

export async function deleteCategory(id: number): Promise<void> {
  return invoke("delete_category", { id });
}

export async function deleteTag(id: number): Promise<void> {
  return invoke("delete_tag", { id });
}

export async function getProjectNotes(projectId: number): Promise<ProjectNote[]> {
  return invoke("get_project_notes", { projectId });
}

export async function createNote(projectId: number, content: string): Promise<number> {
  return invoke("create_note", { input: { project_id: projectId, content } });
}

export async function batchArchiveProjects(ids: number[]): Promise<void> {
  return invoke("batch_archive_projects", { ids });
}

export async function batchDeleteProjects(ids: number[]): Promise<void> {
  return invoke("batch_delete_projects", { ids });
}

export async function batchRestoreProjects(ids: number[]): Promise<void> {
  return invoke("batch_restore_projects", { ids });
}

export async function batchMoveCategory(ids: number[], categoryId: number | null): Promise<void> {
  return invoke("batch_move_category", { ids, categoryId });
}

export async function translate(text: string, targetLang: string): Promise<string> {
  return invoke("translate", { text, targetLang });
}

export async function detectTextLanguage(text: string): Promise<string> {
  return invoke("detect_text_language", { text });
}

export async function shouldTranslate(sourceLang: string | null, targetLang: string): Promise<boolean> {
  return invoke("should_translate", { sourceLang, targetLang });
}

export async function saveRunbook(id: number, content: string): Promise<void> {
  return invoke("save_runbook", { id, content });
}

export async function getRunbook(id: number): Promise<string | null> {
  return invoke("get_runbook", { id });
}

export interface Translations {
  translated_summary: string | null;
  translated_use_cases: string | null;
  translated_risks: string | null;
  translated_dependencies: string | null;
  translated_description: string | null;
  readme_translation: string | null;
}

export async function saveTranslations(
  id: number,
  language: string,
  translated_summary: string | null,
  translated_use_cases: string | null,
  translated_risks: string | null,
  translated_dependencies: string | null,
  translated_description: string | null
): Promise<string> {
  return invoke("save_translations", { id, language, translated_summary, translated_use_cases, translated_risks, translated_dependencies, translated_description });
}

export async function getTranslations(id: number): Promise<Translations | null> {
  return invoke("get_translations", { id });
}

export async function saveReadmeTranslation(id: number, translation: string): Promise<void> {
  return invoke("save_readme_translation", { id, translation });
}

export async function clearTranslations(id: number): Promise<void> {
  return invoke("clear_translations", { id });
}

export interface RefreshReadmeResult {
  readme_content: string | null;
  variants_count: number;
}

export async function refreshProjectReadme(projectId: number): Promise<RefreshReadmeResult> {
  return invoke("refresh_project_readme", { projectId });
}

export interface ProjectClone {
  id: number;
  project_id: number;
  cloned_path: string | null;
  cloned_at: string | null;
  proxy_enabled: boolean;
  proxy_protocol: string;
  proxy_host: string;
  proxy_port: number;
  proxy_username: string;
  proxy_password: string | null;
}

export async function getCloneSettings(projectId: number): Promise<ProjectClone> {
  return invoke("get_clone_settings", { projectId: projectId });
}

export async function saveCloneSettings(clone: ProjectClone): Promise<void> {
  return invoke("save_clone_settings", { clone });
}

export async function buildCloneCommand(projectId: number, gitUrl: string, projectName: string): Promise<string> {
  return invoke("build_clone_command", { projectId, gitUrl, projectName });
}

export interface Vault {
  name: string;
  path: string;
  project_count: number;
}

export interface VaultValidation {
  valid: boolean;
  error?: string;
  table_count?: number;
  project_count?: number;
}

export interface VaultConfig {
  path: string;
}

export interface AppSettings {
  theme: string;
  default_language: string;
  auto_translate_readme: boolean;
  auto_translate_report: boolean;
  download_path: string;
  default_editor: string;
  llm_provider: string;
  llm_api_base: string;
  llm_api_key: string;
  llm_model: string;
  llm_available_models?: string[];
  llm_proxy_enabled: boolean;
  llm_proxy_protocol: string;
  llm_proxy_host: string;
  llm_proxy_port: number;
  llm_proxy_username: string;
  llm_proxy_password: string;
  proxy_enabled: boolean;
  proxy_protocol: string;
  proxy_host: string;
  proxy_port: number;
  proxy_username: string;
  proxy_password: string;
  crawler_enabled: boolean;
  crawler_api_url: string;
  readme_update_frequency: string;
  translate_engine: string;
  google_api_key: string;
  google_proxy_enabled: boolean;
  google_proxy_protocol: string;
  google_proxy_host: string;
  google_proxy_port: number;
  google_proxy_username: string;
  google_proxy_password: string;
}

export async function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function getVaultConfig(): Promise<VaultConfig> {
  return invoke("get_vault_config");
}

export async function saveVaultConfig(config: VaultConfig): Promise<void> {
  return invoke("save_vault_config", { config });
}

export async function getVaultsRoot(): Promise<string> {
  return invoke("get_vaults_root");
}

export async function createVault(name: string, path: string): Promise<Vault> {
  return invoke("create_vault", { name, path });
}

export async function migrateOldData(vaultPath: string, oldDbPath: string): Promise<number> {
  return invoke("migrate_old_data", { vaultPath, oldDbPath });
}

export async function getOldDbPath(): Promise<string> {
  return invoke("get_old_db_path");
}

export async function listVaults(): Promise<Vault[]> {
  return invoke("list_vaults");
}

export async function openVault(path: string): Promise<Vault> {
  return invoke("open_vault", { path });
}

export async function deleteVault(path: string, permanent: boolean): Promise<void> {
  return invoke("delete_vault", { path, permanent });
}

export async function validateVault(path: string): Promise<VaultValidation> {
  return invoke("validate_vault", { path });
}

export async function getCurrentVault(): Promise<Vault | null> {
  return invoke("get_current_vault");
}

export async function importVault(name: string, path: string): Promise<Vault> {
  return invoke("import_vault", { name, path });
}

export async function executeClone(projectId: number, gitUrl: string, projectName: string): Promise<string> {
  return invoke("execute_clone", { project_id: projectId, git_url: gitUrl, project_name: projectName });
}

export interface LlmDebugInfo {
  llm_configured: boolean;
  api_key_len: number;
  api_key_preview: string;
  api_base: string;
  model: string;
  proxy_enabled: boolean;
  error: string | null;
}

export async function debugLlmStatus(): Promise<LlmDebugInfo> {
  return invoke("debug_llm_status");
}

export interface TestLlmResult {
  success: boolean;
  elapsed_ms: number;
  response_preview: string;
  error: string | null;
}

export async function testLlmDirect(projectId: number): Promise<TestLlmResult> {
  return invoke("test_llm_direct", { projectId });
}
