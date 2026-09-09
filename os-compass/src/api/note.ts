import { invoke } from '@tauri-apps/api/core';

export interface ProjectNote {
  id: number;
  project_id: number;
  content: string;
  created_at: string;
  updated_at: string;
}

export interface CreateNoteInput {
  project_id: number;
  content: string;
}

export interface UpdateNoteInput {
  id: number;
  content: string;
}

export const noteApi = {
  async getByProject(projectId: number): Promise<ProjectNote[]> {
    return invoke('get_project_notes', { projectId });
  },

  async create(input: CreateNoteInput): Promise<number> {
    return invoke('create_note', { input });
  },

  async update(input: UpdateNoteInput): Promise<void> {
    return invoke('update_note', { input });
  },

  async delete(id: number): Promise<void> {
    return invoke('delete_note', { id });
  },
};
