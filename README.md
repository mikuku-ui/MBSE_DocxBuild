# MBSE 测试子系统

面向 **MBSE 大系统中测试侧**的一套独立子系统（Rust axum + React + PostgreSQL）。
围绕 **需求 → 测试项 → 测试用例 → 测试记录 → 测试报告** 五阶段逐级追踪，把追溯结果
（追溯矩阵 / 覆盖分析 / 基线）和测试文档一起组织、填写并**装配成文档**。

数据流（核心思路）：

```
台账编排（体系：定义这类项目要哪些数据）   ← 可复用 / 可多套 / 字段 + 自定义表列
        │
        ▼ 项目绑定一套体系
项目管理（建/看/改项目 + 全量项目数据）
        │
        ▼ 执行清单 = 唯一填数面（值存「项目级一份」，全项目文档共享）
文档模板（五原语引用体系字段/表 + 追溯切片 + 人工槽）    ← 模板只是数据消费者
        │
        ▼ 清单补全后装配
文档（正文：字段插值 / 带自定义列头的数据表 / 追溯矩阵 / 槽内容）
```

## 模块

| 前端页面 | 路由 | 说明 |
| --- | --- | --- |
| 需求追踪 | `/` | 五阶段泳道画布；类型化链接（`derives_from` / `verifies` 等单一规范方向）；列内纵向排序（画布拖拽 / 上移下移 / 一键整理）；追溯链 / 影响面 / 矩阵 / 覆盖读取期现算不落库；数据按项目隔离 |
| 文档模板 | `/templates` | 3 步 Tabs：① 文档结构（章/节/段落/表格… 层级树 + 画布，**全局共享**）② 编排内容（五原语 `content_spec`：文本 / 字段 / 数据表 / 追溯切片 / 人工槽）③ 装配预览（选项目+实例即预览成文） |
| 执行清单 | `/executions` | 选「当前项目 + 全局模板 + 执行阶段」→ 按模板文档序摊开待办；**项目数据在此填写**（唯一填数面），人工槽逐项填正文内容 |
| 台账编排 | `/schemes` | 体系设计器：给「这类项目」定义要哪些**标量字段**（带 doc_kind 作用域）与**表**（自定义列），可多套复用 |
| 项目管理 | `/projects` | 项目 CRUD + 绑定体系（更换会清空该项目数据）+ 展开行**全量查看/修改项目数据**；顶栏右上角快捷切换当前项目 |
| 测试项编写 | `/test-items` | 为需求创建 `test_item` 节点并 `derives_from` 所选需求 |

后端已删去旧「台账」链路：职责拆成 **体系（结构定义）** + **项目数据（值，项目级一份）**。

## 技术栈

| 端 | 技术 |
| --- | --- |
| 后端 | Rust（axum 0.8 + sqlx 0.8 / PostgreSQL），分层 `domain ← application ← api / infrastructure` |
| 前端 | Vite + React 18 + TypeScript + antd v5 + @xyflow/react v12 + Redux Toolkit（Tailwind v4） |
| 数据库 | PostgreSQL，库 `MBSETestLib`，schema `mbsetest`，迁移**追加式**（sqlx checksum 不可变） |

## 仓库布局

```
.
├── backend/                     # Rust 后端（crate: mbse-testlib-backend）
│   ├── src/
│   │   ├── domain/              # 实体 + repository port（trace/document/execution/scheme/projectdata/project）
│   │   ├── application/         # 服务层 + 派生纯函数 + 装配引擎 + ServiceError
│   │   ├── api/http/            # 路由 + handler + dto + 错误映射
│   │   └── infrastructure/      # sqlx 仓储 + 连接池 + 追加式迁移 migrations/0001-0010
│   ├── dev-seed-trace.json      # 五阶段 demo（23 节点 + 19 边），POST /api/trace/ingest 灌入
│   └── Cargo.toml
├── frontend/                    # Vite + React + antd + reactflow
│   └── src/features/
│       ├── trace/               # 需求追踪画布
│       ├── template/            # 模板三 Tabs（结构 / 五原语编排 / 装配预览）
│       ├── execution/           # 执行清单（含「项目数据」填数区）
│       ├── scheme/              # 台账编排（体系设计器）
│       ├── projectdata/         # 项目数据通用编辑器（项目页 + 执行清单共用）
│       └── project/  testitem/  # 项目管理 / 测试项编写
├── knowledge-base/              # 上一项目（task-dashboard）的经验沉淀：领域决策 / 坑（保留）
└── config.toml                  # 后端运行配置示例（端口 / database.url 覆盖，空则用 DATABASE_URL）
```

## 快速开始

前置：本机可连到一台 PostgreSQL（默认 `127.0.0.1:5432`）。

```sql
-- 1) 建库（用 superuser）
CREATE DATABASE "MBSETestLib";

-- 2) PG 15+ 加固集群常见坑：public 对该角色不可写时，给角色自有 schema 并指 search_path
--    （本套迁移/查询全用未限定表名，落到第一个 schema；开发库即此配置：dev/dev → schema mbsetest）
CREATE SCHEMA IF NOT EXISTS mbsetest AUTHORIZATION <user>;
ALTER ROLE <user> SET search_path TO mbsetest, public;
```

```bash
# 3) 后端（首次启动自动应用迁移 0001-0010 并种子默认体系「大纲测评」+ 默认示例项目）
cp backend/.env.example backend/.env   # 填好 DATABASE_URL
cd backend && cargo run                # http://127.0.0.1:38123

# 4) 前端
cd frontend && pnpm install && pnpm dev # http://localhost:5173（/api 代理到 38123）
```

验证：

```bash
cargo check && cargo test              # backend：单元测试（内存桩，无需数据库）
pnpm build                             # frontend：tsc + vite 构建
curl http://127.0.0.1:38123/api/health # 健康检查
```

## 数据库模型（迁移里程碑）

`backend/src/infrastructure/database/migrations/` 追加式演进：

- **0001–0003** trace：`trace_nodes`（`UNIQUE(external_source, external_ref)`）/ `trace_links`（类型化 + 禁自环）/ `trace_baselines`（快照）
- **0004–0005** document 模板图（模板/节点/边 + `content_spec` jsonb）、execution 清单（实例/项）
- **0006** `trace_nodes.sort_key`：阶段列内纵向排序持久化
- **0007–0008** 项目化：`projects` + trace/execution 各加 `project_id`（数据按项目隔离；模板仍全局）
- **0009–0010** 旧「台账」→ 拆分：`schemes/scheme_fields/scheme_tables/scheme_table_columns`（体系定义）
  + `project_field_values/project_table_rows`（项目数据，值按项目存一份）+ `projects.scheme_id`

派生量（追溯链 / 影响面 / 覆盖 / 执行顺序 / 文档序）一律**读取期现算，不落库**。

## 文档

- `knowledge-base/`：上一项目沉淀（实体扁平化、派生不落库、依赖图/环、层级布局等，及 pitfalls）。
- 当前 README 早期更细的领域设计（两图模型、需求节点约定、ConTeXt 预留接口等）见 git 历史中的旧版。
