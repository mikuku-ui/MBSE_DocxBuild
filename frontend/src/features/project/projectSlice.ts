import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import type { PayloadAction } from '@reduxjs/toolkit';
import { api } from '../../app/api';
import type { Project } from '../../app/types';

export interface ProjectState {
  projects: Project[];
  /** 当前选中的项目（追踪页数据按它收口）；null = 尚未选（提示新建/选择） */
  currentProjectId: string | null;
  loading: boolean;
  error: string | null;
}

const initialState: ProjectState = {
  projects: [],
  currentProjectId: null,
  loading: false,
  error: null,
};

export const loadProjects = createAsyncThunk('project/list', async () =>
  api.get<Project[]>('/projects'),
);

export const createProject = createAsyncThunk(
  'project/create',
  async (body: { name: string; scheme_id?: string | null }): Promise<Project> =>
    api.post('/projects', { name: body.name, scheme_id: body.scheme_id ?? null }),
);

export const renameProject = createAsyncThunk(
  'project/rename',
  async (body: { id: string; name: string }): Promise<Project> =>
    api.put(`/projects/${body.id}`, { name: body.name }),
);

/** 绑定/解绑体系（= 选择/更换执行方式）。后端换体系会清空该项目已填数据。 */
export const bindProjectScheme = createAsyncThunk(
  'project/bindScheme',
  async (body: { id: string; scheme_id: string | null }): Promise<Project> =>
    api.put(`/projects/${body.id}/scheme`, { scheme_id: body.scheme_id }),
);

export const deleteProject = createAsyncThunk(
  'project/delete',
  async (id: string): Promise<{ deleted: boolean }> => api.del(`/projects/${id}`),
);

const projectSlice = createSlice({
  name: 'project',
  initialState,
  reducers: {
    setCurrentProjectId(state, action: PayloadAction<string | null>) {
      state.currentProjectId = action.payload;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(loadProjects.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(loadProjects.fulfilled, (state, action) => {
        state.loading = false;
        state.projects = action.payload;
      })
      .addCase(loadProjects.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message ?? '加载项目失败';
      })
      .addCase(createProject.fulfilled, (state, action) => {
        if (!state.projects.some((p) => p.id === action.payload.id)) {
          state.projects.push(action.payload);
        }
        if (!state.currentProjectId) state.currentProjectId = action.payload.id;
      })
      .addCase(renameProject.fulfilled, (state, action) => {
        const i = state.projects.findIndex((p) => p.id === action.payload.id);
        if (i >= 0) state.projects[i] = action.payload;
      })
      .addCase(bindProjectScheme.fulfilled, (state, action) => {
        const i = state.projects.findIndex((p) => p.id === action.payload.id);
        if (i >= 0) state.projects[i] = action.payload;
      })
      .addCase(deleteProject.fulfilled, (state, action) => {
        const id = action.meta.arg;
        state.projects = state.projects.filter((p) => p.id !== id);
        if (state.currentProjectId === id) {
          state.currentProjectId = state.projects[0]?.id ?? null;
        }
      });
  },
});

export const { setCurrentProjectId } = projectSlice.actions;
export default projectSlice.reducer;
