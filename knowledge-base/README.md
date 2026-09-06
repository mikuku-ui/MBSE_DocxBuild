# 可复用知识库 · workflow + task

> 从 `task-dashboard` 蒸馏出来的“**下一项目可照搬**”资产，放在仓库根目录 `knowledge-base/`。
> 定位：**业务会变，技术可复用** → 本篇把分量放在**技术实现核心 + 踩坑清单**上；具体业务语义只留“够你对着改”的简要说明，不铺细节。
> 核对源码锚点：HEAD `9febb97`，`main` 分支。

## 📦 资产总览

| 优先级 | 文档 | 是什么 |
|---|---|---|
| ★★★ 先读 | [core-technical-core.md](core-technical-core.md) | **可复用技术核心**：分层/port 仓储、语义枚举、派生不落库、DAG+防环+Kahn、记忆化递归、视图分离、确定性自动布局、sqlx 迁移规矩（T1–T10） |
| ★★★ 先读 | [pitfalls.md](pitfalls.md) | **踩坑清单**：建模坑 P1–P11（层级诱惑、双事实源、派生落库、环检测方向、reference 混图…）+ 环境/DB 坑 P12–P17（sqlx checksum、rolinherit、pg_dump 导入、Windows 进程/psql） |
| ★★ 参考 | [01](01-core-entity-flat-tasknode.md) … [07](07-canvas-layout-and-visual-groups.md) | 领域语义简要（背景，业务可改）：核心实体、语义状态、依赖图、execution_state、dependency_health、拓扑顺序、画布布局 |

> 想要“1 小时把 workflow+task 的骨架移植到新项目”：读 `core-technical-core.md` 的 T 清单 → 对着 `pitfalls.md` 避雷 → 需要某块业务语义再翻 01–07。

---

## 这套技术到底解决什么问题（30 秒版）

- **核心只留一个扁平实体 TaskNode**，任务关系 = 一张 **dependency DAG**。
- “能不能开始 / 做完了没 / 上游变了要不要复查 / 按什么顺序执行 / 画布怎么摆”全部是**读取期派生**，库里不存任何 `depth/order/health/state`。
- 分层单向：`domain ← application ← api`；sqlx 藏在 infrastructure；外部格式（md 同步器、SDK 桥）只依赖 domain。
- 副产品：几乎每条规则都有**不连库的内存单测**，规则可信、可迁移。

详见 [core-technical-core.md](core-technical-core.md) 的模型图与速查表。

---

## 领域语义简要（业务会变，仅作对照）

| 文档 | 一句话 | 对应源码 |
|---|---|---|
| [01 核心实体](01-core-entity-flat-tasknode.md) | 扁平 TaskNode；进度/状态正交；完成只看语义枚举 | `domain/task/*`、`application/task/service.rs` |
| [02 状态模型](02-status-model-semantic-kind.md) | 5 个语义枚举驱动规则；名称/外观是数据；守卫删引用/删末类 | `domain/status/entity.rs`、`application/status/service.rs` |
| [03 依赖图](03-dependency-graph-and-cycles.md) | dependency vs reference；边不变式；建边防环 | `domain/relationship/*`、`application/workflow/service.rs` |
| [04 execution_state](04-derived-execution-state.md) | 6 态派生优先级，未开始分 blocked/ready | `api/http/dto.rs` |
| [05 dependency_health](05-derived-dependency-health.md) | healthy/warning/blocked；时间戳扰动传播（简版+正确规则） | `application/workflow/service.rs` |
| [06 执行顺序](06-execution-order-topological.md) | Kahn 拓扑；只走 dependency；created_at 起排 | `application/workflow/service.rs` |
| [07 画布布局/视觉分组](07-canvas-layout-and-visual-groups.md) | 视图状态分离；弱连通切工作流；自动 DAG 布局 | `frontend/.../layout.ts`、`application/workflow/layout.rs` |

> 文档里的 R1–R14 是“当前业务约定的实现”，换业务时可改；**支撑它们的技术结构（T1–T10）才是不变的**。

---

## 目录与边界说明
- 位置：`<repo-root>/knowledge-base/`（与 `backend/`、`frontend/` 平级）。
- 只收逻辑与经验，不收：具体表结构、HTTP 契约、SSE/字节格式——需要时回看源码即可。
- 若下一项目要做“本地控制器/md 同步器连 web”，它应落在 `integrations` 层、只依赖 domain —— 这套分层已为你留好边界（见 core T1）。
