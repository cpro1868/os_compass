import { invoke } from '@tauri-apps/api/core';

export interface Category {
  id: number;
  name: string;
  path: string;
  explain?: string;
  sort_order: number;
  parent_id?: number;
  is_system: boolean;
  created_at: string;
}

export interface CreateCategoryInput {
  name: string;
  path: string;
  explain?: string;
  sort_order?: number;
  parent_id?: number;
}

export const categoryApi = {
  async getAll(): Promise<Category[]> {
    return invoke('get_categories');
  },

  async getById(id: number): Promise<Category | null> {
    return invoke('get_category', { id });
  },

  async create(input: CreateCategoryInput): Promise<number> {
    return invoke('create_category', { input });
  },

  async update(category: Category): Promise<void> {
    return invoke('update_category', { category });
  },

  async delete(id: number): Promise<void> {
    return invoke('delete_category', { id });
  },
};
