# 01 · 核心实体：扁平 TaskNode（Flat TaskNode）

> 源码锚点：`backend/src/domain/task/entity.rs` · `backend/src/domain/task/value.rs`

## 1. 结构不变式

TaskNode 是系统**唯一**持久化业务实体，且刻意做成**扁平、独立**：

```
TaskNode {
    id                 // UUID，标识符，永不复用
    title              // 文本
    description        // 文本
    status_id          // → TaskStatus（唯一的“完成/未完成”事实源）
    progress           // 值对象 {current, target}，与 status 正交
    priority           // Low | Medium | High（纯展示，不参与规则）
    status_changed_at  // 最近一次“状态变更”时间戳（健康度判据，见 05）
    created_at         // 创建时间
    updated_at         // 任意更新都会刷新
}
```

不存在：`parent_id`、`children`、`root_id`、层级/深度字段。
> 为什么？层级会把“图”伪装成“树”，迫使你维护 parent/children 一致性；本项目选择用**有向依赖图**表达任务间语义，层次只是一种视图，不需要字段支撑。

### R1 · 完成与否只看状态语义
`is_completed() == (semantic_kind == Completed)`。
- 没有平行的 `completed` 布尔 → 不可能出现“状态是 Todo 但被标成已完成”的矛盾。
- Cancelled / Failed **不是**完成。

## 2. 进度值对象（Progress）

```rust
TaskProgress { current: u32, target: u32 }
```

- 不变式：`current >= 0`、`target > 0`。
- 进度与状态**正交**（R2）：
  - 进度到 100% 不会自动把任务改为完成；
  - 反过来，改状态也不会动进度。
  - 需要“进度 100% = 完成”的产品，请把它放到**上层应用层**（`application`）做成用例，不要写进实体构造器。

## 3. 创建 / 更新的应用层规则（R3）

来源：`backend/src/application/task/service.rs`

| 操作 | 规则 |
|---|---|
| create | `title.trim()` 非空（否则 Validation）；`status_id` 缺省时取**第一个** `not_started` 语义的状态；`progress`/`priority` 默认 `None`（可选，之后可补） |
| update | 任意字段可改；**只有当 `status_id` 实际变化**时才刷新 `status_changed_at`（普通 edit 不碰它） |
| delete | 有仓储删除即可；下游依赖/健康度由“不存在任务”被自然忽略（见 03 悬挂边、05 记忆化递归） |

> 可移植要点：把 `status_changed_at` 的刷新和“判断完成/阻塞/健康”一起读，规则才会闭环。别把该时间戳当成“updated_at”。

## 4. 迁移到下一项目的最小骨架

```text
[TaskNode 值语义]             [应用用例]                 [外层]
 title 非空                   create(默认 not_started)   → ServiceError
 progress{cur>=0,tgt>0}       update(仅状态变化刷时间)     Validation/NotFound/Conflict
 completed = kind==Completed  delete                      Repository
```

- 实体 + 值对象放在 domain，不 import 任何 DB/HTTP 类型。
- 仓储用 trait（port），sqlx 实现放 infrastructure，测试用内存 map 桩。
