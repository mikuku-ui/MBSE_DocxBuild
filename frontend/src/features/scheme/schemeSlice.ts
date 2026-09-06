import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import type { PayloadAction } from '@reduxjs/toolkit';
import { api } from '../../app/api';
import type { Scheme, SchemeFull, SchemeSpec } from '../../app/types';

export interface SchemeState {
  /** 体系元数据清单（设计页左侧 / 项目绑体系下拉用）。 */
  list: Scheme[];
  /** 当前正在设计的体系 id（null = 未选中）。 */
  currentId: string | null;
  /** 当前体系的完整定义（含 id；编辑期整存替换的底）。 */
  current: SchemeFull | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
}

const initialState: SchemeState = {
  list: [],
  currentId: null,
  current: null,
  loading: false,
  saving: false,
  error: null,
};

export const listSchemes = createAsyncThunk('scheme/list', async () =>
  api.get<Scheme[]>('/schemes'),
);

export const loadScheme = createAsyncThunk(
  'scheme/load',
  async (id: string): Promise<SchemeFull> => api.get(`/schemes/${id}`),
);

export const createScheme = createAsyncThunk(
  'scheme/create',
  async (spec: SchemeSpec): Promise<SchemeFull> => api.post('/schemes', spec),
);

export const saveScheme = createAsyncThunk(
  'scheme/save',
  async (body: { id: string; spec: SchemeSpec }): Promise<SchemeFull> =>
    api.put(`/schemes/${body.id}`, body.spec),
);

export const deleteScheme = createAsyncThunk(
  'scheme/delete',
  async (id: string): Promise<{ deleted: boolean }> => api.del(`/schemes/${id}`),
);

const schemeSlice = createSlice({
  name: 'scheme',
  initialState,
  reducers: {
    setCurrentSchemeId(state, action: PayloadAction<string | null>) {
      state.currentId = action.payload;
    },
    resetSchemeError(state) {
      state.error = null;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(listSchemes.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(listSchemes.fulfilled, (state, action) => {
        state.loading = false;
        state.list = action.payload;
        // 当前选中的体系若已被删/不存在 → 落空（不硬切，避免误伤编辑态）
        if (state.currentId && !state.list.some((s) => s.id === state.currentId)) {
          state.currentId = null;
          state.current = null;
        }
      })
      .addCase(listSchemes.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message ?? '加载体系失败';
      })
      .addCase(loadScheme.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(loadScheme.fulfilled, (state, action) => {
        state.loading = false;
        state.current = action.payload;
        state.currentId = action.payload.scheme.id;
        const i = state.list.findIndex((s) => s.id === action.payload.scheme.id);
        if (i >= 0) state.list[i] = action.payload.scheme;
      })
      .addCase(loadScheme.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message ?? '加载体系定义失败';
      })
      .addCase(createScheme.fulfilled, (state, action) => {
        state.saving = false;
        state.current = action.payload;
        state.currentId = action.payload.scheme.id;
        if (!state.list.some((s) => s.id === action.payload.scheme.id)) {
          state.list.push(action.payload.scheme);
        }
      })
      .addCase(saveScheme.pending, (state) => {
        state.saving = true;
        state.error = null;
      })
      .addCase(saveScheme.fulfilled, (state, action) => {
        state.saving = false;
        state.current = action.payload;
        const i = state.list.findIndex((s) => s.id === action.payload.scheme.id);
        if (i >= 0) state.list[i] = action.payload.scheme;
      })
      .addCase(saveScheme.rejected, (state, action) => {
        state.saving = false;
        state.error = action.error.message ?? '保存体系失败';
      })
      .addCase(createScheme.pending, (state) => {
        state.saving = true;
        state.error = null;
      })
      .addCase(createScheme.rejected, (state, action) => {
        state.saving = false;
        state.error = action.error.message ?? '新建体系失败';
      })
      .addCase(deleteScheme.fulfilled, (state, action) => {
        state.list = state.list.filter((s) => s.id !== action.meta.arg);
        if (state.currentId === action.meta.arg) {
          state.currentId = null;
          state.current = null;
        }
      });
  },
});

export const { setCurrentSchemeId, resetSchemeError } = schemeSlice.actions;
export default schemeSlice.reducer;
