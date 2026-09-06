# 04 · 派生状态：execution_state

> 源码锚点：`backend/src/api/http/dto.rs`（DTO 组装层计算，**永不落库**）

## 1. 思想

- `execution_state` **不是**字段，而是每次读取时由 `semantic_kind + dependency_health` 算出来的**派生量**。
- 因此永远不会出现“库里存的 state 跟当前图不一致”的脏数据。
- 六个取值：`ready` `running` `blocked` `completed` `failed` `cancelled`（没有独立的 `pending`）。

## 2. 判定优先级（自上而下，命中即停）

```text
                SemanticKind / 健康度
                       │
  ┌────────────────────┼───────────────────────┐
  │                    │                        │
Failed ──────────► "failed"                     │
Completed ───────► "completed"                  │
InProgress ──────► "running"   （不管健康度）     │
Cancelled ───────► "cancelled"                  │
NotStarted ───────► 看 dependency_health:
                       ├─ health == Blocked  → "blocked"
                       └─ 否则               → "ready"
```

### R9 · 优先级顺序
`failed > completed > running > cancelled > (not_started → blocked/ready)`

### R10 · NotStarted 的二分
- 未开始 **且存在未完成前置**（依赖被挡）→ `blocked`；
- 未开始 **且前置都完成 / 无前置** → `ready`。

> `running`（进行中）**不受健康度影响**：进行中的任务不会显示为 blocked——它已在执行，阻塞与否只看它停在哪里。

## 3. 与健康度的关系

| execution_state | 前提（综合） |
|---|---|
| `blocked` | 仅当 `NotStarted` 且 `dependency_health == Blocked` |
| `ready` | `NotStarted` 且前置健康（无未完成前置） |
| `running` | `InProgress`（无论健康） |
| `failed` / `completed` / `cancelled` | 各自 `SemanticKind` 唯一决定 |

> 注意：一个 **Completed 但有上游变动**的任务，其 `execution_state` 仍是 `completed`（它做完了），只是在**健康度**上标 `warning`（见 05）。二者是不同维度的两个灯。

## 4. 搬运要点
- 把这段判定做成一个**纯函数** `execution_state(kind, health) -> &str`，不要散落在各 DTO。
- 让“本任务是否完成 / 是否有未完成前置”都走同一个 helper，避免 04 和 05 各写一套判断导致不一致。
