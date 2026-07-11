export interface Project {
  id: number;
  name: string;
  url: string | null;
  source: string;
  source_id: string | null;
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
  readme_lang: string | null;
  health_score: number | null;
  ai_summary: string | null;
  ai_use_cases: string | null;
  ai_risks: string | null;
  ai_dependencies: string | null;
  translated_summary: string | null;
  translated_use_cases: string | null;
  translated_risks: string | null;
  translated_dependencies: string | null;
  translated_description: string | null;
  readme_translation: string | null;
  category_id: number | null;
  lifecycle_status: string;
  data_status: string;
  is_downloaded: boolean;
  local_path: string | null;
  archived_at: string | null;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface Category {
  id: number;
  name: string;
  path: string;
  explain: string | null;
  sort_order: number;
  parent_id: number | null;
  is_system: boolean;
  created_at: string;
  updated_at: string;
}

export interface Tag {
  id: number;
  name: string;
  color: string | null;
  source: "ai" | "user" | "preset";
  created_at: string;
}

export interface ProjectNote {
  id: number;
  project_id: number;
  content: string;
  created_at: string;
  updated_at: string;
}

export interface SystemVariable {
  key: string;
  value: string | null;
  isSecret: boolean;
  createdAt: string | null;
  updatedAt: string | null;
}

export interface VariableDef {
  key: string;
  name: string;
  description?: string;
  secret: boolean;
  default?: string;
}

export interface SourcePlugin {
  id: string;
  name: string;
  pluginClass: string;
  description: string | null;
  enabled: boolean;
  version: string | null;
  requiredVariables: VariableDef[];
  createdAt: string | null;
  updatedAt: string | null;
}

export type LifecycleStatus = "TO_EXPLORE" | "DIVING" | "IN_USE" | "ABANDONED";

export const LIFECYCLE_STATUS_LABELS: Record<LifecycleStatus, string> = {
  TO_EXPLORE: "待探索",
  DIVING: "深入了解",
  IN_USE: "使用中",
  ABANDONED: "已弃用",
};

export const LIFECYCLE_STATUS_COLORS: Record<LifecycleStatus, string> = {
  TO_EXPLORE: "bg-gray-100 text-gray-700",
  DIVING: "bg-blue-100 text-blue-700",
  IN_USE: "bg-green-100 text-green-700",
  ABANDONED: "bg-red-100 text-red-700",
};

export function parseLanguages(languages: string | null): string[] {
  if (!languages) return [];
  try {
    const parsed = JSON.parse(languages);
    if (Array.isArray(parsed)) {
      return parsed.filter((l: any) => typeof l === "string");
    }
    if (typeof parsed === "string") {
      return [parsed];
    }
  } catch {
    if (typeof languages === "string" && languages.trim()) {
      return [languages.trim()];
    }
  }
  return [];
}
