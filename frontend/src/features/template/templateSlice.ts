import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import { api } from '../../app/api';
import type {
  DocumentTemplate,
  TemplateEdge,
  TemplateFull,
  TemplateNode,
} from '../../app/types';

export interface TemplateState {
  templates: DocumentTemplate[];
  current: TemplateFull | null;
  loading: boolean;
  error: string | null;
}

const initialState: TemplateState = {
  templates: [],
  current: null,
  loading: false,
  error: null,
};

export const loadTemplates = createAsyncThunk('template/list', async () =>
  api.get<DocumentTemplate[]>('/documents/templates'),
);

export const loadTemplateFull = createAsyncThunk(
  'template/full',
  async (id: string): Promise<TemplateFull> => api.get(`/documents/templates/${id}`),
);

export const createTemplate = createAsyncThunk(
  'template/create',
  async (body: { name: string; kind: string; description?: string | null }) =>
    api.post<DocumentTemplate>('/documents/templates', body),
);

export const deleteTemplate = createAsyncThunk('template/delete', async (id: string) =>
  api.del(`/documents/templates/${id}`),
);

export const createNode = createAsyncThunk(
  'template/createNode',
  async (body: {
    template_id: string;
    node_type: string;
    title: string;
    sort_key?: number;
  }) =>
    api.post<TemplateNode>(`/documents/templates/${body.template_id}/nodes`, {
      node_type: body.node_type,
      title: body.title,
      sort_key: body.sort_key,
    }),
);

export const deleteNode = createAsyncThunk(
  'template/deleteNode',
  async (body: { template_id: string; node_id: string }) =>
    api.del(`/documents/templates/${body.template_id}/nodes/${body.node_id}`),
);

export const updateNode = createAsyncThunk(
  'template/updateNode',
  async (body: {
    template_id: string;
    node_id: string;
    title?: string;
    node_type?: string;
    /** 五原语内容声明；传入则整体覆写该节点 content_spec（传 {} = 清空） */
    content_spec?: Record<string, unknown>;
  }) =>
    api.put<TemplateNode>(
      `/documents/templates/${body.template_id}/nodes/${body.node_id}`,
      {
        ...(body.title !== undefined ? { title: body.title } : {}),
        ...(body.node_type !== undefined ? { node_type: body.node_type } : {}),
        ...(body.content_spec !== undefined ? { content_spec: body.content_spec } : {}),
      },
    ),
);

export const createEdge = createAsyncThunk(
  'template/createEdge',
  async (body: {
    template_id: string;
    source_node_id: string;
    target_node_id: string;
    edge_type?: string;
  }) =>
    api.post<TemplateEdge>(`/documents/templates/${body.template_id}/edges`, {
      source_node_id: body.source_node_id,
      target_node_id: body.target_node_id,
      edge_type: body.edge_type ?? 'contains',
    }),
);

export const deleteEdge = createAsyncThunk(
  'template/deleteEdge',
  async (body: { template_id: string; edge_id: string }) =>
    api.del(`/documents/templates/${body.template_id}/edges/${body.edge_id}`),
);

/** 批量重排：逐条覆写 sort_key（导航窗格拖拽 / 画布同组纵向拖拽共用）。 */
export const reorderNodes = createAsyncThunk(
  'template/reorderNodes',
  async (body: { template_id: string; orders: Array<{ id: string; sort_key: number }> }) =>
    api.put<TemplateNode[]>(`/documents/templates/${body.template_id}/nodes/order`, {
      orders: body.orders,
    }),
);

const templateSlice = createSlice({
  name: 'template',
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    builder
      .addCase(loadTemplates.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(loadTemplates.fulfilled, (state, action) => {
        state.loading = false;
        state.templates = action.payload;
      })
      .addCase(loadTemplateFull.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(loadTemplateFull.fulfilled, (state, action) => {
        state.loading = false;
        state.current = action.payload;
      })
      .addCase(loadTemplates.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message ?? '加载失败';
      })
      .addCase(loadTemplateFull.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message ?? '加载失败';
      });
  },
});

export default templateSlice.reducer;
