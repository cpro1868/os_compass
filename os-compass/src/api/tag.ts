import { invoke } from '@tauri-apps/api/core';

export interface Tag {
  id: number;
  name: string;
  color?: string;
  source: string;
  created_at: string;
}

export interface CreateTagInput {
  name: string;
  color?: string;
  source?: string;
}

export const tagApi = {
  async getAll(): Promise<Tag[]> {
    return invoke('get_tags');
  },

  async getByProject(projectId: number): Promise<Tag[]> {
    return invoke('get_project_tags', { projectId });
  },

  async create(input: CreateTagInput): Promise<number> {
    return invoke('create_tag', { input });
  },

  async addToProject(projectId: number, tagId: number): Promise<void> {
    return invoke('add_tag_to_project', { projectId, tagId });
  },

  async removeFromProject(projectId: number, tagId: number): Promise<void> {
    return invoke('remove_tag_from_project', { projectId, tagId });
  },

  async delete(id: number): Promise<void> {
    return invoke('delete_tag', { id });
  },
};
