import { configureStore } from '@reduxjs/toolkit';
import executionReducer from '../features/execution/executionSlice';
import projectReducer from '../features/project/projectSlice';
import schemeReducer from '../features/scheme/schemeSlice';
import templateReducer from '../features/template/templateSlice';
import traceReducer from '../features/trace/traceSlice';

// 组合根：trace / template / execution / scheme（台账编排 = 项目执行方式设计，全局可复用）
// + projects（项目维度：绑定体系；追踪/执行/项目数据按项目隔离，模板全局通用）。
export const store = configureStore({
  reducer: {
    trace: traceReducer,
    template: templateReducer,
    execution: executionReducer,
    scheme: schemeReducer,
    projects: projectReducer,
  },
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
