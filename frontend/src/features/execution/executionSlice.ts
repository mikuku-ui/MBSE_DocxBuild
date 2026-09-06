import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import { api } from '../../app/api';
import type {
  DocumentTemplate,
  ExecutionDetail,
  ExecutionInstance,
  ExecutionItem,
} from '../../app/types';

export interface ExecutionState {
  /** 文档模板全局通用（不分项目），一次拉取后所有项目共用 */
  templates: DocumentTemplate[];
  /** 当前项目的执行实例（切项目后重拉） */
  instances: ExecutionInstance[];
  detail: ExecutionDetail | null;
  loading: boolean;
  error: string | null;
}

const initialState: ExecutionState = {
  templates: [],
  instances: [],
  detail: null,
  loading: false,
  error: null,
};

/** 执行页数据面：全局模板 + 当前项目的执行实例。 */
export const loadExecSetup = createAsyncThunk(
  'execution/setup',
  async (projectId: string) => {
    const [templates, instances] = await Promise.all([
      api.get<DocumentTemplate[]>('/documents/templates'), // 模板全局，不带 project
      api.get<ExecutionInstance[]>(`/executions/instances?project_id=${projectId}`),
    ]);
    return { templates, instances };
  },
);

export const createExecution = createAsyncThunk(
  'execution/create',
  async (body: {
    project_id: string;
    template_id: string;
    stage?: string | null;
  }): Promise<ExecutionInstance> => api.post('/executions/instances', body),
);

export const loadExecutionDetail = createAsyncThunk(
  'execution/detail',
  async (id: string): Promise<ExecutionDetail> =>
    api.get(`/executions/instances/${id}`),
);

export const updateExecutionItem = createAsyncThunk(
  'execution/updateItem',
  async (body: { id: string; status?: string; content?: Record<string, unknown> }) =>
    api.put<ExecutionItem>(`/executions/items/${body.id}`, {
      status: body.status,
      content: body.content,
    }),
);

const executionSlice = createSlice({
  name: 'execution',
  initialState,
  reducers: {
    // 切换项目前清掉上一项目的实例/详情（配合 loading，避免旧项目数据闪现）
    resetForProject(state) {
      state.instances = [];
      state.detail = null;
      state.error = null;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(loadExecSetup.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(loadExecSetup.fulfilled, (state, action) => {
        state.loading = false;
        state.templates = action.payload.templates;
        state.instances = action.payload.instances;
      })
      .addCase(loadExecutionDetail.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(loadExecutionDetail.fulfilled, (state, action) => {
        state.loading = false;
        state.detail = action.payload;
      })
      .addCase(loadExecSetup.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message ?? '加载失败';
      })
      .addCase(loadExecutionDetail.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message ?? '加载失败';
      });
  },
});

export const { resetForProject } = executionSlice.actions;
export default executionSlice.reducer;
