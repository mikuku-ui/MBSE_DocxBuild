# 05 · 派生状态：dependency_health（业务语义简版）

> 完整技术：记忆化递归的写法与原理见 [`core-technical-core.md`](core-technical-core.md) T6。
> 这里只留“它到底算什么”的业务定义，好让下一个项目对着规则改阈值/时机。源码锚点：`backend/src/application/workflow/service.rs::compute_health_map`。

## 三档语义（一句话）

| 档 | 触发 | 通俗说 |
|---|---|---|
| `healthy` | 默认 | 绿灯 |
| `blocked` | **本任务未完成**，且存在**未完成的直接 dependency 前置** | 红灯：被卡住 |
| `warning` | **本任务已完成**，但某个“上游扰动时刻”**晚于它的确认时刻** `status_changed_at` | 黄灯：做完后上游又动了，可能要复查 |

两档派生灯与自身状态是**两个维度**：Completed + warning = “做完了但要复查”，其 execution_state 仍是 `completed`（见 04）。

## 计算规则（精确版）

对每个节点 X，遍历 X 的所有 **dependency 直接前置** P：

```text
for P in 直接前置(X):
    if P 任务不存在:         跳过（悬挂免疫）
    if P 未完成:
        disturbance = max(disturbance, P.status_changed_at)
        if X 未完成:  blocked = true
    else (P 已完成):
        (P_health, P_disturb) = eval(P)            // 记忆化递归
        if P_health == Warning:
            disturbance = max(disturbance, P_disturb)   // 只有 Warning 链继续上抛

X.health = blocked ? Blocked
         : (X 已完成 && disturbance > X.status_changed_at) ? Warning
         : Healthy
```

要点（技术而非业务的部分请移步 core T6）：
- **已完成任务**才可能 warning；判据就是一次时间比较 `disturbance > X.status_changed_at`。
- “上游变动”的传播是：未完成前置贡献它自己的 `status_changed_at`；已完成前置只在**它自己是 Warning**时把扰动继续上抛。
- 改 X 状态不会自动改下游，只刷新 X 自己的 `status_changed_at` —— 下游是否变 warning 由比较自然推出（这正是“派生不落库”的收益，见 core T4/P3）。

## 反直觉但正确（业务护栏，可改）
- reopen 上游（已完成→未完成）：会让“确认晚于那次变动”的下游 Completed 任务变 warning——这是刻意的“你做完的东西可能过时了”。
- 想消除 warning：把上游那条未完成线做完，或把该任务 reopen 后再确认一次（刷新 `status_changed_at`）。
- reference 永远不参与健康度。
