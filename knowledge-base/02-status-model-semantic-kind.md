# 02 · 状态模型：语义类型驱动（Semantic-Kind Driven）

> 源码锚点：`backend/src/domain/status/entity.rs` · `backend/src/application/status/service.rs` · `backend/src/api/http/dto.rs`

## 1. 一张图看懂

```
      状态名(自定义, 如"Todo"/"进行中"/"已完成")  ←── 业务可自建
            │ has-a
            ▼
     SemanticKind（5 个内置语义）
  ┌──────────────┬─────────────┬─────────────┬────────────┬──────────┐
  │  NotStarted  │  InProgress │  Completed  │  Cancelled │  Failed  │
  └──────────────┴─────────────┴─────────────┴────────────┴──────────┘
  is_completed?    否             否          ★是★          否         否
```

## 2. 铁律（系统规则的唯一判据）

> **所有系统规则只读 `SemanticKind` 这一个枚举——绝不读状态的名字或外观。**
> 名字 "Done" 与 "已完成" 都不算数；只有 `kind == Completed` 才算完成。

`SemanticKind` 的完整集合（五类，新增内置语义才需动枚举）：

| Kind | 含义 | 是否完成 |
|---|---|---|
| `NotStarted` | 未开始 | 否 |
| `InProgress` | 进行中 | 否 |
| `Completed` | 已完成 | **是** |
| `Cancelled` | 已取消 | 否 |
| `Failed` | 已失败 | 否 |

- `Appearance`（颜色/图标等）是**纯展示**字段，塞进任何规则判定都是坏味道。
- 同一语义类型可有**多个**状态名（例如多个自定义“未开始”变体），它们共享同一套规则。

## 3. 状态是任务的一个属性，不是状态机

本项目刻意**不实现状态机/受限流转**：

- **任意流转（R4）**：任务可从 Completed 改回 InProgress / NotStarted（手动 reopen），不设守卫。
  > 若你的产品需要“已完成不可 reopen / 只能按顺序流转”，请在 `application` 层加一个 **transition policy**，不要塞进 domain 实体。
- 换状态时由应用层刷新 `status_changed_at`（见 01 §3）——这是健康度判据的时间戳（见 05）。

## 4. 状态目录（自定义状态）的 CRUD 守卫

来源：`application/status/service.rs`

### R5 · 创建 / 更新
- `name`：`trim()` 后**非空**；在同一系统内**唯一**（重复 → Conflict）。

### 删除的两道守卫
| 守卫 | 说明 | 报错 |
|---|---|---|
| **被引用** | 仍有任务 `status_id` 指向它 → 拒删 | Conflict |
| **类型最后一个** | 该 `SemanticKind` 下只剩它一个状态 → 拒删（保证每类语义至少存活一个可用状态） | Conflict |

> 设计原因：任务新建时需要一个“默认 not_started”，且历史数据必须总能解释。
> 搬运时保留这两条，否则删光某类状态会让旧任务失去合法解释。

## 5. 状态如何驱动派生状态

`SemanticKind` 是 04 / 05 两张派生表的**唯一输入**：

```text
SemanticKind
   ├─ Completed → execution_state = completed；参与“warning 传播”判定
   ├─ Failed    → execution_state = failed
   ├─ Cancelled → execution_state = cancelled
   ├─ InProgress→ execution_state = running（health 不影响它）
   └─ NotStarted→ execution_state 取决于 dependency_health：blocked / ready
```
