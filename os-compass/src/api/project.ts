import { invoke } from '@tauri-apps/api/core';

export interface Project {
  id: number;
  name: string;
  url: string;
  source: string;
  lifecycle_status: string;
  data_status: string;
  stars: number;
  languages: string;
  description?: string;
  readme_versions: string;
  readme_selected_lang: string;
  readme_translations: string;
  readme_hash?: string;
  readme_last_fetched?: string;
  summary?: string;
  applicable_scenarios?: string;
  category_id?: number;
  is_downloaded: boolean;
  local_path?: string;
  health_score?: number;
  license?: string;
  runbook?: string;
  archived_at?: string;
  deleted_at?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateProjectInput {
  name: string;
  url: string;
  source: string;
  description?: string;
  category_id?: number;
}

export interface UpdateProjectInput {
  id: number;
  name?: string;
  url?: string;
  lifecycle_status?: string;
  category_id?: number;
}

export const projectApi = {
  async getAll(dataStatus?: string): Promise<Project[]> {
    return invoke('get_projects', { dataStatus });
  },

  async getById(id: number): Promise<Project | null> {
    return invoke('get_project', { id });
  },

  async create(input: CreateProjectInput): Promise<number> {
    return invoke('create_project', { input });
  },

  async update(input: UpdateProjectInput): Promise<void> {
    return invoke('update_project', { input });
  },

  async updateStatus(id: number, lifecycleStatus: string): Promise<void> {
    return invoke('update_project_status', { id, lifecycleStatus });
  },

  async delete(id: number, permanent = false): Promise<void> {
    return invoke('delete_project', { id, permanent });
  },
};
