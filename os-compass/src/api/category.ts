import { invoke } from '@tauri-apps/api/core';

export interface Category {
  id: number;
  name: string;
  path: string;
  explain?: string;
  sort_order: number;
  parent_id?: number;
  is_system: boolean;
  project_count: number;
  created_at: string;
}

export interface CreateCategoryInput {
  name: string;
  path: string;
  explain?: string;
  sort_order?: number;
  parent_id?: number;
}

export interface PreviewItem {
  full_path: string;
  status: "new" | "skip" | "error";
  reason?: string | null;
  source_line: number;
}

export interface ParsedCategories {
  total: number;
  items: PreviewItem[];
}

export interface ImportError {
  source_line: number;
  full_path: string;
  reason: string;
}

export interface ImportSummary {
  created: number;
  skipped: number;
  errors: ImportError[];
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

  async parseFile(filePath: string, parentId: number | null): Promise<ParsedCategories> {
    return invoke('parse_categories_file', { filePath, parentId });
  },

  async importFile(filePath: string, parentId: number | null): Promise<ImportSummary> {
    return invoke('import_categories_from_file', { filePath, parentId });
  },
};
