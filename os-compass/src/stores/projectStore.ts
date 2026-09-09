import { create } from 'zustand';
import { Project, projectApi, CreateProjectInput, UpdateProjectInput } from '../api/project';

interface ProjectState {
  projects: Project[];
  currentProject: Project | null;
  loading: boolean;
  error: string | null;
  
  fetchProjects: (dataStatus?: string) => Promise<void>;
  fetchProject: (id: number) => Promise<void>;
  createProject: (input: CreateProjectInput) => Promise<number>;
  updateProject: (input: UpdateProjectInput) => Promise<void>;
  updateProjectStatus: (id: number, status: string) => Promise<void>;
  deleteProject: (id: number, permanent?: boolean) => Promise<void>;
  setCurrentProject: (project: Project | null) => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  projects: [],
  currentProject: null,
  loading: false,
  error: null,

  fetchProjects: async (dataStatus?: string) => {
    set({ loading: true, error: null });
    try {
      const projects = await projectApi.getAll(dataStatus);
      set({ projects, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  fetchProject: async (id: number) => {
    set({ loading: true, error: null });
    try {
      const project = await projectApi.getById(id);
      set({ currentProject: project, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  createProject: async (input: CreateProjectInput) => {
    set({ loading: true, error: null });
    try {
      const id = await projectApi.create(input);
      await get().fetchProjects();
      return id;
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  updateProject: async (input: UpdateProjectInput) => {
    set({ loading: true, error: null });
    try {
      await projectApi.update(input);
      await get().fetchProjects();
      if (get().currentProject?.id === input.id) {
        await get().fetchProject(input.id);
      }
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },

  updateProjectStatus: async (id: number, status: string) => {
    try {
      await projectApi.updateStatus(id, status);
      await get().fetchProjects();
      if (get().currentProject?.id === id) {
        await get().fetchProject(id);
      }
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
  },

  deleteProject: async (id: number, permanent = false) => {
    try {
      await projectApi.delete(id, permanent);
      await get().fetchProjects();
      if (get().currentProject?.id === id) {
        set({ currentProject: null });
      }
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
  },

  setCurrentProject: (project: Project | null) => {
    set({ currentProject: project });
  },
}));
