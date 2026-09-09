import { create } from "zustand";
import type { Project, Category, Tag } from "../types";
import * as api from "../api";

interface AppState {
  projects: Project[];
  categories: Category[];
  tags: Tag[];
  loading: boolean;
  error: string | null;
  
  fetchProjects: () => Promise<void>;
  fetchCategories: () => Promise<void>;
  fetchTags: () => Promise<void>;
  addProject: (input: Partial<Project>) => Promise<void>;
  updateStatus: (id: number, status: string) => Promise<void>;
  removeProject: (id: number) => Promise<void>;
  addCategory: (name: string, parentId?: number) => Promise<void>;
  deleteCategory: (id: number) => Promise<void>;
  addTag: (name: string, color?: string) => Promise<void>;
  deleteTag: (id: number) => Promise<void>;
}

export const useAppStore = create<AppState>((set, get) => ({
  projects: [],
  categories: [],
  tags: [],
  loading: false,
  error: null,

  fetchProjects: async () => {
    set({ loading: true, error: null });
    try {
      const projects = await api.getProjects();
      set({ projects, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  fetchCategories: async () => {
    try {
      const categories = await api.getCategories();
      set({ categories });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchTags: async () => {
    try {
      const tags = await api.getTags();
      set({ tags });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addProject: async (input) => {
    set({ loading: true });
    try {
      await api.createProject(input);
      await get().fetchProjects();
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  updateStatus: async (id, status) => {
    try {
      await api.updateProjectStatus(id, status);
      await get().fetchProjects();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  removeProject: async (id, permanent = false) => {
    try {
      await api.deleteProject(id, permanent);
      await get().fetchProjects();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addCategory: async (name, parentId) => {
    try {
      await api.createCategory({ name, parent_id: parentId, sort_order: 0 });
      await get().fetchCategories();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteCategory: async (id) => {
    try {
      await api.deleteCategory(id);
      await get().fetchCategories();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addTag: async (name, color) => {
    try {
      await api.createTag({ name, color });
      await get().fetchTags();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteTag: async (id) => {
    try {
      await api.deleteTag(id);
      await get().fetchTags();
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
