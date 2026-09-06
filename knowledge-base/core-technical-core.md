# 可复用技术实现核心（Technical Core）

> 这一篇是整套 `workflow + task` **能落地、能移植**的“技术骨架”。业务字段可能变，下面这些**结构与算法**不变。
> 每一节都给：技术点 → 为什么 → 最小实现锚点。全文语言无关，锚点标 Rust/TS 文件路径。

---

## T1 · 分层 + 依赖方向单一（这决定能否复用）

```text
        ┌──────────────────────────────┐
        │            api / http        │   DTO 组装、错误映射；HTTP 不碰查询
        │              │               │
        │        application / service │   用例编排 + 规则 + 领域事务
        │              │               │
        │      domain / 实体+值对象+port│   ★ 核心资产；不 import DB/HTTP 类型
        └──────────┬────┴────┬─────────┘
                   │         │
   infrastructure/sqlx      integrations（md/文件…）依赖 domain，绝不反向
```
- domain 定义**仓储 trait（port）**；sqlx 实现放 infrastructure；应用层只认 trait。
- 于是**规则可以在不连数据库的情况下单测**——仓库里每套规则都配了内存版 repo 桩测试，这就是“规则可信”的来源。
- 下层永不 import 上层。http/dto 里 import domain 是合法的，domain 里出现 `reqwest`/`serde_json` 之外的东西就要警惕。

## T2 · 领域错误不绑 HTTP：一个 4 变体 ServiceError

application 只抛 `ServiceError`，由 api 层统一映射成状态码：

| 变体 | 语义 | → HTTP |
|---|---|---|
| `NotFound` | 资源不存在 | 404 |
| `Validation` | 输入/不变式违例（空标题、source==target、非法进度…） | 400 |
| `Conflict` | 状态冲突（重复边、会成环、重名、删被引用/最后一个…） | 409 |
| `Repository` | 包一层基础设施错误，**不透 sqlx 类型**到接口 | 500 |

## T3 · 状态建模：语义类型枚举（closed-set）驱动一切

- 建一个**封闭枚举**（5 个语义：NotStarted/InProgress/Completed/Cancelled/Failed），序列化走 snake_case。
- **所有规则 match 这个枚举，绝不 match 状态名 / 颜色 / 图标**；名字与外观是“数据”，用户可自由建。
- 用户自定义状态 = 一行 `(name, semantic_kind, appearance)` 数据；系统只关心 `semantic_kind`。
- `is_completed()` 一个实现点：`kind == Completed`。**没有平行的 `completed: bool`** → 矛盾态在结构上不可能出现。
- 派生展示（execution_state / 图标）也从枚举推导，见 T6。

> 反模式：把“完成”做成任务上独立的布尔位，等于引入第二个事实源，迟早出现“completed=true 但状态是 Todo”并需要靠 bugfix 去圆。

## T4 · “派生量不落库，读取时计算” + 时间戳变更检测

最值钱的一条技术决定：
- `execution_state`、`dependency_health`、执行顺序 `order`、画布自动布局 **全部是读取期派生的**，数据库里没有 `depth/order/health/state` 列。
- 好处：**不可能出现派生值与真实图不一致**（无需缓存失效策略）；删/改任务后全图自动一致；快照/恢复直接可用。
- 配套一个“变更检测”时间戳：`status_changed_at` 只在 **status_id 真正变化**时刷新（不是普通 edit）。它让“上游在我完成之后又动过 → 提示复查”成为纯时间比较，无需持久化下游状态。

```rust
// task/service.rs update() 里的关键一行——只在实际换状态时打点
if status_id != node.status.id {
    node.status = <新状态>;
    node.mark_status_changed();   // 刷新 status_changed_at
}
node.mark_updated();              // updated_at 每次更新都刷
```

## T5 · DAG：一个边类型承担“执行语义”，建边即防环

- 关系拆成两档：`Dependency`（有执行语义：排序/健康/布局用它）与 `Reference`（纯关联）。**规则只把 dependency 喂进任何图算法**，reference 永远不参与 → 不同机制口径统一，不会互相打架。
- **在创建边时做环检测**，让“现有图恒为 DAG”成为不变量——这是下面所有算法（拓扑、记忆化递归）能安全终止的前提。
- 环检测技术（Rust 锚点 `application/workflow/service.rs::would_create_cycle`）：要加 `source→target`，只需在**不含新边**的图上 DFS：从 `target` 出发沿 dependency 出边能否到达 `source`。能 → 会闭合 → 409。

## T6 · 在图上做“带记忆化的递归派生”

健康度这种“每个节点依赖其前置、前置又依赖更前”的量，最干净的实现是**带 memo 的递归**（锚点 `compute_health_map`）：

```text
eval(node) -> (health, disturbance):
    对每个直接 dependency 前置 P:
        未完成 P → disturbance = max(disturbance, P.status_changed_at)
                    （仅当本节点未完成）→ blocked
        已完成 P → 递归 eval(P)；仅当 P 是 Warning 才把 P 的 disturbance 继续上抛
    health = blocked ? Blocked
           : (本节点已完成 && disturbance > status_changed_at) ? Warning : Healthy
```
- 因为图是无环的（T5 不变量），递归**必然终止**；memo 让整体接近 O(V+E)，而不是每个下游都重算一遍上游。
- 只从 completed-Warning 节点继续上抛，语义与算法都简洁（见文档 05 的一句版，业务会变，这里记技术）。

## T7 · 拓扑执行顺序：Kahn，且“确定性”优先

```rust
// 前置先出、依赖者后出；同一批“可执行”按 created_at 排序 → 稳定可复现
let indegree = 每个任务的 dependency 前置数;
let mut ready = indegree==0 的任务按 created_at(再按 id) 排序;
while let Some(t) = ready.pop_front() {
    order.push(t);
    for next in outgoing[t] { if --indegree[next]==0 { ready.push(next); } }
}
```
- 悬挂边（端点任务已删）**构边时跳过**，排序永不因脏数据崩溃。
- 确定性（同输入→同输出）是为了 diff/缓存/导出可比——刻意排序 tie-break。

## T8 · 视图状态与业务状态分离（画布）

- 画布坐标、用户视觉分组属于**视图状态**：各自 repo、各自保存，**不参与任何业务规则**。
- 视觉分组=纯容器（`group(title)+members`）。服务端只做两件守卫：标题非空、**成员必须真实存在**（防止幽灵节点）——其余一律不管业务。
- 批量坐标保存用 **last-write-wins upsert**；删除分组 = 只删容器行（FK cascade 删 members），任务/关系分毫不动。
- 想要“锁住某节点不自动排”，在**前端状态**加标记即可，后端无需建字段（视图状态不进 domain）。

## T9 · 前端：自动布局做成纯函数（确定性、可单测）

锚点 `frontend/src/features/workflow/layout.ts`。两件事分开：

1. **切工作流**：把依赖图当无向图求**弱连通分量**（只数 dependency 边；reference 永不合并）；孤立任务 = 单点。分量按组内最早 created_at 排序后**纵向堆叠**，`GROUP_GAP_Y > NODE_GAP_Y` 让组界清晰。
2. **单个 DAG 内布局**：
   - 分层（只定 X）：sink 距离法 `dist=0`（无出边依赖）否则 `1+max(succ dist)`；列号 = `maxDist − dist`（汇合靠右）。`x = level×(W+gapX)`。
   - 定 Y（减交叉 + 对齐）：初始列序 title→id；**双向 barycenter** 扫 N 轮重排同列；再 **Y 松弛**迭代（左向朝前置重心、右向朝后继重心拉，`newY = oldY·(1−α)+idealY·α` 防震荡；相邻层权重 1、跨层权重 0.5）；列内最小间距冲突消解用“正扫向下顶 + 反扫向上顶取均值”，避免整体持续下移。

## T10 · 迁移（sqlx 风格）：不可变 + IF NOT EXISTS

- 全部 `CREATE TABLE IF NOT EXISTS`；迁移文件一经应用**绝不修改**（sqlx 对文件字节做 sha384 校验，改了会让启动报 checksum 错，见踩坑 P4）。
- 新增变更 = 追加新的编号迁移，而不是改旧的。

---

## 一张“技术使用清单”带走

| 你要做的功能 | 用上面哪条 |
|---|---|
| 建任务实体 | T1/T2/T3/T4 |
| 表示“任务能不能开始/做完了吗” | T3 枚举 + T4 读取期派生 |
| 任务前后置关系 | T5 DAG + T7 排序 |
| 自动提示“上游变了要不要复查” | T4 时间戳 + T6 记忆化递归 |
| 画布整整齐齐/能手动摆 | T8 视图分离 + T9 纯函数布局 |
| 给下一项目留后门（md 同步器等） | T1 integrations 只依赖 domain |
