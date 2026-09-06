# MBSE 测试子系统（需求五阶段追踪 + 文档模板 + 执行清单）

面向 **MBSE 大系统里测试人员侧** 的一套子系统：与大系统保持**独立性**（数据联动）——
研发人员在研发系统导出数据（或测试人员依据被测文档手工录入），测试人员把结果导入本系统，
做「需求 → 测试项 → 测试用例 → 测试记录 → 测试报告」的**逐级追踪**，并把追溯结果以
**追溯矩阵**等形态渲染成文档章节（ConTeXt）。本仓库是其核心数据与应用的脚手架：
**架构 + 代码框架 + 核心库表（`MBSETestLib`）**。

> 定位约束：整套 MBSE 里我们只占一小块，且**没有历史包袱**。因此可以借鉴
> IBM DOORS/DNG 的语义（类型化对象、类型化链接、追溯矩阵、覆盖分析、基线），
> 但实现保持轻、干净，并处处考虑对大系统的**隔离**（见下）。

---

## 一、两个核心建模决定

1. **两张彼此独立的图**（不共用一套模型）：
   - **需求追踪图（Trace）** ＝ 大 MBSE 系统灌入本子系统的**数据入口**。它是一份
     干净投影：节点用 `external_source + external_ref` 回链大系统，支持**幂等批量摄取
     / 整体替换**；派生视图（追溯链/影响面/矩阵/覆盖）读取期现算，**不落库**。
   - **文档模板图（Document）** ＝ 章节编排。节点 = 文档结构元素（章/节/段落/表格/
     图/附录…），节点互挂成边，串起来 = 一篇文档模板。执行时**摊成清单（task 列表化）**
     给执行人员逐项填写。
2. **桥接只发生在渲染期**：模板节点有一种 `trace_view` 类型，其 `content_spec` 描述
   「对追踪图做何种派生」，渲染时派生成追溯表/章节。无 FK 强耦合 → 追踪图可整体替换，
   文档模板不受影响。

追踪域按 DOORS/DNG 语义建模：节点类型 = **五阶段泳道**（最右为顶层需求，越靠左越是下一阶段）
`software_requirement`（需求）→ `test_item`（测试项）→ `test_case`（测试用例）→
`test_record`（测试记录）→ `test_report`（测试报告，最左）；类型化链接 + **单一规范方向**
（`source`=下级/覆盖方，`target`=上级/被覆盖方；如 `测试项 --derives_from--> 需求`）。其中
**测试报告是「对需求的回答」，只连测试记录、不直连需求**（`测试报告 --derives_from--> 测试记录`，
报告据记录作答）；每个需求对应报告里的一条回答，该回答连到**这个需求派生的全部记录**。于是
「报告↔需求」的作答关系不画边、不落库，由 记录→用例→测试项→需求 的链向上推导即得。其余提供
追溯矩阵、覆盖分析、基线快照。
画布上左/右侧端点连线只发生在**跨列（跨阶段）**节点间；同列内上下节点无连接需求，节点也只能在
本列内纵向移动，不会拖进其它阶段列。列内纵向顺序由节点自带 `sort_key`（同阶段内 0..n-1 序数，
落库保存）决定，用户三种方式改顺序，全部持久化：**画布拖拽**松手即按落点整列重编号；**左栏 ↑/↓**
微调相邻项；右上角 **「整理」按钮** 按需求序一键规范重排——需求越靠前，其派生链上的
测试项/用例/记录/报告越靠前，一个节点对应多个需求时锚定最靠前的那个，无需求可达的孤儿节点排最
后。新增节点落在本阶段最底。重摄取（幂等 upsert）不改写已排好的 `sort_key`。

### 项目（追踪数据的隔离边界）

追踪图不是一张全局图，而是**按项目隔离**（0007）：`trace_nodes` / `trace_links` /
`trace_baselines` 各带 `project_id`，幂等键从全局 `(external_source, external_ref)` 改为
**项目内唯一** `(project_id, external_source, external_ref)`；系统启动种入固定 id 的
「默认示例项目」（`00000000-0000-4000-8000-000000000001`），存量演示数据全部归入它。

- 新建项目得到一张**空白追踪面板**（五阶段泳道仍显示，只是没有节点）；项目之间数据互不可见，
  删除项目会**级联清空**该项目下的追踪数据。
- 前端顶栏**右上角**只做**快捷切换**（纯切换，无管理弹窗）；项目的新增 / 重命名 / 删除
  集中在「项目管理」页（删除**级联清空**该项目下的追踪 + 执行数据）。
- 需求只经**导入**产生（`POST /api/trace/requirements/upload`）；追踪页只展示并支持连线 /
  排序 / 删除。独立「测试项编写」页为需求创建测试项：每个测试项 = 追踪图一个 `test_item`
  节点并 `derives_from` 所选需求（可多选），不改动文档模板。
- 所有 trace 接口都要求 `?project_id=<uuid>`；**执行清单同样按项目隔离**（0008，
  `execution_instances` 挂 `project_id`，列表接口需 `?project_id=`）。**文档模板是唯一
  全局共享资源**，不归任何项目管。

### 需求节点模型约定（数据联动的输入形态）

研发与测试在不同阶段用不同系统、甚至不都是模型化交付（有人只交文档），所以这套子系统
**不做法定的大系统集成**，只做**数据导入减负**。测试人员拿到的可能是结构化需求、也可能是
文档——像 DOORS 一样，人把文档筛出具体需求，产物收敛成**同一份需求节点 JSON**。约定：

- 通用要素**只定四条**：`id`（↔ `external_source + external_ref` 幂等键）、`title`（标题，
  来源可自带"章节号+标题"）、`description`（内容/描述）、`module`（软分组）。
- 章节号、章节格式这类**来源专有、各家不一**的内容**不做强约束**：一律落进 `attributes`
  透传（本系统只透传不建模，schema 归来源侧）。约束宁可少不要多——过度约束会让系统难用。
- **不做文档解析**；文档转需求由人完成，导入入口统一走
  `POST /api/trace/requirements/upload`（研发导出 JSON → 需求节点，幂等 upsert，
  顶层只写一次 `source`）。
- 反向的**测试结构 JSON 导出**（本系统内容导出给别处）本期**不做、只作预留项**，见 §八。

---

## 二、技术栈

| 端 | 技术 |
| --- | --- |
| 后端 | Rust（axum 0.8 + sqlx 0.8 / PostgreSQL + 分层：domain ← application ← api / infrastructure） |
| 前端 | Vite + React 18 + TypeScript + **antd** + **@xyflow/react v12** + Redux Toolkit |
| 数据库 | PostgreSQL，库名 **`MBSETestLib`**，迁移追加式（不可变） |

上一项目（task-dashboard）的技术经验沉淀在 `knowledge-base/`，本实现直接套用其
分层与「派生不落库 / 视图与数据分离 / 图模型扁平化」的教训。

---

## 三、仓库布局

```
MBSE_DocxBuild/
├── knowledge-base/          # 上一项目经验（保留）
├── ConTeXt/                 # 已有 ConTeXt 文档管线（tex 组件的命名锚点）
├── config.toml              # 后端运行配置（server 端口 38123；database.url 覆盖）
├── backend/                 # Rust 后端（crate: mbse-testlib-backend）
│   └── src/
│       ├── main.rs
│       ├── app/             # 配置加载 + bootstrap 组合根
│       ├── domain/          # trace / document / execution 实体 + repository port
│       ├── application/     # 三组服务 + 追踪派生纯函数 + ServiceError
│       ├── api/http/        # routes + 三组 handler + dto + 错误映射
│       ├── infrastructure/  # sqlx Postgres 仓储 + 连接池 + 迁移（migrations/0001-0008.sql）
│       └── integrations/    # ConTeXt 导出 port（预留 stub，见 §六）
└── frontend/                # Vite + antd + reactflow
    └── src/
        ├── app/             # store / api client / 类型契约 / 路由壳
        └── features/
            ├── trace/       # 需求追踪画布（reactflow）
            ├── template/    # 文档模板编辑器画布（reactflow）
            └── execution/   # 执行清单（待办 + 内容落库）
```

---

## 四、数据库表（`MBSETestLib`，迁移追加式）

| 迁移 | 表 | 作用 |
| --- | --- | --- |
| 0001 | `trace_nodes` | 追踪图节点（`UNIQUE(external_source, external_ref)` 幂等键；attributes 透传） |
| 0002 | `trace_links` | 追踪边（有向 + `link_type` + `UNIQUE(source,target,link_type)` + 禁自环） |
| 0003 | `trace_baselines` | 基线快照（`snapshot_json` 冻结副本，不做查询源） |
| 0004 | `document_templates` / `document_template_nodes`（`tex_component` 预留列）/ `document_template_edges` | 文档模板图 |
| 0005 | `execution_instances` / `execution_items` | 执行清单（内容落库，结构派生） |
| 0006 | `trace_nodes.sort_key` | 同阶段列内纵向排序序数（0..n-1 落库，支撑画布拖拽 / 左栏上移下移 / 整理 的持久化） |
| 0007 | `projects` + trace 三表各加 `project_id` | 项目隔离：种入「默认示例项目」并回填存量；幂等键收敛为项目内唯一；FK 级联删除 |
| 0008 | `execution_instances` 加 `project_id` | 执行清单项目化：回填存量到默认项目；FK `projects(id)` 级联删除 |

派生量（追溯链 / 影响面 / 覆盖 / 执行顺序 / 模板节点顺序）一律**读取期现算，不落库**。

---

## 五、运行

### 1. 后端

```bash
# 1) 准备数据库（PostgreSQL 监听 5432）
#    用你本机可用的 superuser 执行：
CREATE DATABASE "MBSETestLib";

# 2) 连接串：复制 backend/.env.example 为 backend/.env 并填写，或用 config.toml database.url
DATABASE_URL=postgres://<user>:<pass>@127.0.0.1:5432/MBSETestLib

# 本机说明（PG 15+ 加固集群常见坑）：若 public schema 对该角色不可写
# （报 `permission denied for schema public`），给角色一个自有 schema 并把
# search_path 指过去即可——本套迁移与查询全部用未限定表名，会落到第一个 schema：
#   CREATE SCHEMA IF NOT EXISTS mbsetest AUTHORIZATION <user>;
#   ALTER ROLE <user> SET search_path TO mbsetest, public;
# （当前开发库即如此配置：角色 dev/dev，schema `mbsetest`。）

# 3) 启动（自动跑迁移 0001-0008）
cd backend
cargo run          # 监听 http://127.0.0.1:38123
```

测试与静态检查：`cd backend && cargo check && cargo test`（内存桩单测，无需数据库）。

### 2. 前端

```bash
cd frontend
pnpm install
pnpm dev           # http://localhost:5173 ，/api 代理到 38123
```

五个页面：**需求追踪**（五阶段泳道画布；数据按**项目**隔离、顶栏右上快捷切换——只展示并
连线 / 排序 / 删除 / 溯源 / 影响面 / 矩阵 / 覆盖，需求经导入新增）、**文档模板**（画布编辑 +
节点增删排序 + `trace_view` 预留，**全局共享**不归项目）、**执行清单**（选全局模板 + 当前项目
+ 执行阶段 → 按模板文档层级摊开待办并逐项填写内容，落在当前项目下）、**测试项编写**（为需求
创建 `test_item` 节点并 `derives_from` 所选需求，不改模板）、**项目管理**（项目列表分页 +
新增 / 重命名 / 删除）。

### 3. 冒烟示例

```bash
# 健康检查
curl http://127.0.0.1:38123/api/health

# 完整五阶段 demo：仓库自带 `backend/dev-seed-trace.json`（23 节点 + 19 边），
# 覆盖各阶段与一条「文档录入需求（req-doc / R-5.4，故意无覆盖）」演示覆盖分析。
# 默认项目 id 固定为 00000000-0000-4000-8000-000000000001（前端首个项目即默认选中）；
# trace 接口都要带 ?project_id=。
curl -X POST "http://127.0.0.1:38123/api/trace/ingest?project_id=00000000-0000-4000-8000-000000000001" \
  -H 'Content-Type: application/json' --data-binary @backend/dev-seed-trace.json

# 最小示例：一条「测试项 derives_from 需求」（测试项设计出来以满足需求）
curl -X POST "http://127.0.0.1:38123/api/trace/ingest?project_id=00000000-0000-4000-8000-000000000001" \
  -H 'Content-Type: application/json' -d '{
  "nodes": [
    {"kind":"software_requirement","external_source":"sr-sys","external_ref":"SR-1","module":"登录","title":"支持账号密码登录"},
    {"kind":"test_item","external_source":"sr-sys","external_ref":"IT-1","module":"登录","title":"账号密码登录校验"}
  ],
  "links": [
    {"source":{"external_source":"sr-sys","external_ref":"IT-1"},
     "target":{"external_source":"sr-sys","external_ref":"SR-1"},
     "link_type":"derives_from"}
  ]
}'

# 上传需求 JSON（研发导出 → 需求节点；source 只声明一次，重复上传幂等覆盖）
curl -X POST "http://127.0.0.1:38123/api/trace/requirements/upload?project_id=00000000-0000-4000-8000-000000000001" \
  -H 'Content-Type: application/json' -d '{
    "source": "reqtool",
    "requirements": [
      {"id":"SR-1","title":"支持账号密码登录","description":"…","module":"登录","section":"3.2.1"}
    ]
  }'

# 追溯矩阵（= 文档「追溯表」章节的渲染源）
curl "http://127.0.0.1:38123/api/trace/matrix?project_id=00000000-0000-4000-8000-000000000001"
```

---

## 六、ConTeXt tex 生成（本轮只钉接口，不实现）

- `document_template_nodes.tex_component`（可空）为预留列。
- `backend/src/integrations/context/ports.rs` 定义导出契约
  `trait ContextExporter { fn export_template(template_id, resolved) -> Result<Vec<ComponentFile>> }`
  与 `ComponentFile{ rel_path, content }`，命名对齐现有 `ConTeXt/` 管线
  （`components/{kind}/chNN-*.tex`、`lua/tables-data-{kind}.lua`、product `.tex`）。
- 节点类型 → tex 骨架映射常量 `NODE_TEX_SKELETON` 已占位（`application/document` 维护，
  `trace_view` 渲染期把追溯表派生结果注入）。
- 迁移接口先钉死，避免日后推翻；**生成器实现留给后续迭代**。

---

## 七、API 摘要

- Projects：`GET/POST /api/projects`、`PUT/DELETE /api/projects/{id}`（改名 / 删除，
  删除级联清空该项目下追踪 + 执行数据）
- Trace：`GET/POST /api/trace/nodes`、`GET/PUT/DELETE /api/trace/nodes/{id}`、
  `GET /api/trace/nodes/{id}/reachable?direction=up|down&link_types=...`、
  `GET/POST /api/trace/links`、`DELETE /api/trace/links/{id}`、
  `POST /api/trace/ingest`（幂等批量摄取：节点+追踪边）、
  `POST /api/trace/requirements/upload`（需求 JSON 上传：source + requirements[] →
  需求节点，见「需求节点模型约定」）、`POST /api/trace/test-items`（测试项编写：校验所选
  均为同项目需求后建 `test_item` 节点 + `derives_from` 边）、`GET /api/trace/coverage`、
  `GET /api/trace/matrix`、`GET/POST /api/trace/baselines`。
  以上 trace 端点**全部要求 `?project_id=<uuid>`**（追踪数据按项目隔离）。
- Documents：`GET/POST /api/documents/templates`、模板 full view / 更新 / 删除、
  节点 CRUD（`/api/documents/templates/{id}/nodes[/{nid}]`）、边 CRUD（全局共享）
- Executions：`GET/POST /api/executions/instances`、实例详情 / 更新、
  清单项更新 `PUT /api/executions/items/{id}`——实例表按项目隔离：列表需
  `?project_id=<uuid>`，创建 body 带 `project_id`（模板仍全局）

错误统一为 `{ "error": "<code>", "message": "…" }`（404/400/409/500）。

---

## 八、本轮不做（后续迭代）

- tex 生成器落地（只留 port + 字段 + 映射常量）。
- 画布坐标持久化 / 手动拖拽分组 / 模板节点「边」的可视化新增。
- **测试结构 JSON 导出（预留项，本期不实现）**：把本系统的执行清单 / 追溯矩阵等测试结构
  导出给外部。内部数据形态已为此保持干净（追溯矩阵读取期派生、执行内容 jsonb 落库），
  日后新增导出接口直接基于这些形态，**无需重构**——做其它功能时注意别让这些形态变脏即可。
- 追踪图与研发侧的自动同步 / 增量刷新协议（当前 `ingest` / `requirements/upload` 为一次性
  批量 upsert 占位）。
- 基线 compare / restore；完整状态机 / 权限 / 审批。
