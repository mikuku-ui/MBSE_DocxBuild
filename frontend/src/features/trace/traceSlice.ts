import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import { api } from '../../app/api';
import type { TraceLink, TraceNode } from '../../app/types';

export interface TraceGraphState {
  nodes: TraceNode[];
  links: TraceLink[];
  loading: boolean;
  error: string | null;
}

const initialState: TraceGraphState = {
  nodes: [],
  links: [],
  loading: false,
  error: null,
};

// 追踪图请求可能因快速切换项目而“后发先至”：用单调序号只认最后一次发起的请求，
// 过期请求的返回 / 错误一律丢弃，避免项目 A 的数据覆盖到当前项目 B。
let latestSeq = 0;

type LoadFailure = { seq: number; message: string };

export const loadTraceGraph = createAsyncThunk(
  'trace/loadGraph',
  async (projectId: string, { rejectWithValue }) => {
    const seq = ++latestSeq;
    try {
      const [nodes, links] = await Promise.all([
        api.get<TraceNode[]>(`/trace/nodes?project_id=${projectId}`),
        api.get<TraceLink[]>(`/trace/links?project_id=${projectId}`),
      ]);
      return { seq, nodes, links };
    } catch (e) {
      return rejectWithValue({
        seq,
        message: e instanceof Error ? e.message : String(e),
      } satisfies LoadFailure);
    }
  },
);

const traceSlice = createSlice({
  name: 'trace',
  initialState,
  reducers: {
    // 切换项目前清空上一项目的图（配合 loading 显示加载态，避免残留旧项目数据闪现）
    clearGraph(state) {
      state.nodes = [];
      state.links = [];
      state.loading = true;
      state.error = null;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(loadTraceGraph.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(loadTraceGraph.fulfilled, (state, action) => {
        if (action.payload.seq !== latestSeq) return; // 过期请求，丢弃
        state.loading = false;
        state.nodes = action.payload.nodes;
        state.links = action.payload.links;
      })
      .addCase(loadTraceGraph.rejected, (state, action) => {
        const payload = action.payload as LoadFailure | undefined;
        if (payload && payload.seq !== latestSeq) return; // 过期请求的错误不覆盖当前
        state.loading = false;
        state.error = payload ? payload.message : action.error.message ?? '加载失败';
      });
  },
});

export const { clearGraph } = traceSlice.actions;
export default traceSlice.reducer;
