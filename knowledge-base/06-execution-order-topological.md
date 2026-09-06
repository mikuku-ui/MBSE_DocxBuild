# 06 · 执行顺序：拓扑排序（Kahn’s Algorithm）

> 源码锚点：`backend/src/application/workflow/service.rs`（`order_by_dependencies`）

## 1. 目标

把任务排成**“被依赖者在前、依赖者在后”的执行清单**（线性化 DAG）。

```
输入：任务集合 + dependency 边
输出：一个稳定顺序 [A, B, C, …]，满足：∀ dependency P→X，P 排在 X 之前
```

- **只走 dependency**：reference 完全不参与（R7）。
- 不是所有任务都连通：相互独立的任务也会进同一个清单，只保证“边约束”满足。

## 2. 算法骨架（R12）

```rust
// indegree = 每个任务“尚未被放进队列的 dependency 前置”数量
let mut indegree = 任务 → 其 dependency 前置数;
let mut queue = 任务中 indegree==0 者;

// ⚠️ 稳定起排：让队列按 created_at 升序（相同再按 id）出队
//    保证同一时刻“都可执行”的任务顺序确定、可复现。
queue.sort_by(created_at, id);

let mut order = [];
while let Some(t) = queue.pop_front() {
    order.push(t);
    for succ in dependency_successors[t] {          // t 的所有直接依赖者
        indegree[succ] -= 1;
        if indegree[succ] == 0 { queue.push(succ); }  // 前置齐了 → 可执行
    }
}
```

- **入度归零才放行** = “前置全部就绪”。
- 队列为空但还剩任务 → 说明有环（正常流程不会发生，因为建边时已禁环，见 03）。

### 语义（source 总在 target 前）
正向 Kahn 的产物是一个满足 **“每个依赖者都排在被依赖者之后”** 的线性清单。UI 上也可把它理解为“从无依赖的源头开始，汇合/收尾节点靠后”。
业务上只需记住一点：

> **任何前缀都构成一个“可先做完再做后面”的集合**——这就是拓扑序能当执行计划的原因。

> 术语提示：`indegree==0` 的“就绪”节点是**没有前置依赖**的任务（源头），不是图论里的叶子；别按字面“叶优先”去理解。

## 3. 悬挂边与未知任务

- 不认识的任务（端点已删）在构图时**跳过该边**，不参与 indegree/successor；
- 孤立任务（无任何 dependency 边）indegree=0，按 created_at 正常进入队列。

## 4. 为什么这样稳定、可移植

| 设计点 | 好处 |
|---|---|
| 只认 dependency | 与健康度/布局共用同一张“业务图”，不会三种机制三套口径 |
| indegree==0 + created_at 起排 | 相同输入 → 相同输出，适合做 diff、缓存、快照恢复 |
| 悬挂边免疫 | 删除任务不会让排序崩溃，配合 03/05 的自愈语义 |

> 搬运提示：若下一个项目需要“同层可并行”，直接在队列出队顺序或产出分组上做文章，别改判定。
