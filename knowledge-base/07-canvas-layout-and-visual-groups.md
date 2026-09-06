# 07 · 画布：自动 DAG 排版 与 视觉分组

> 源码锚点：`frontend/src/features/workflow/layout.ts`（自动排版）· `backend/src/application/workflow/layout.rs`（坐标/分组持久化）· 迁移 `0009_workflow_layout.sql`

## 1. 定位：这是“视图层”不是“业务层”

本模块全部属于**纯画布状态**，与业务语义刻意隔离（R13）：

- 坐标、分组**不影响**任务完成与否、健康度、排序。
- 删除一个视觉分组**只删容器**，不删任务、不动依赖。
- 因此它可整体取舍：想要“自己拉位置 + 分组”就留，不想要就只用自动排版。

```
业务层（01–06）         视图层（本文档）
 TaskNode ───────────►  坐标 positions        （后端持久化，前端直接给 x/y）
 Dependency Graph ───►  自动 DAG 排版          （前端布局算法）
                        视觉分组 visual groups  （纯容器，title + 成员集合）
```

## 2. 视觉分组

后端模型（`application/workflow/layout.rs` + `0009`）：
- `workflow_visual_groups { id, title }` + `workflow_visual_group_members { group_id, task_id }`。
- **组是集合容器**：标题非空；成员 ≥1 且互不重复；成员必须是存在的任务。
- **组 ≠ 树/父子关系**：一个任务可在多个组，也可不在任何组；组不改变依赖语义。
- 删除组 = 级联删除它的成员行（FK `ON DELETE CASCADE`），任务本身无伤。

> 注意与“自动排版的工作流分组（弱连通分量）”区别：那是**布局算法内部概念**，不落库；这里的“视觉分组”是**用户自建、落库**的容器。二者同名不同物。

## 3. 自动排版：两级处理

`layoutWorkflow(nodes, edges)` 返回每个节点的 `{ x, y, level, index }`。

### 第一级 · 把大图切成独立“工作流”（R14）
- 把依赖图当**无向图**求**弱连通分量**；每个连通分量是一个独立工作流。
- 没有 dependency 边的孤立任务 = 单点工作流。
- reference 边**永不合并**两个分量。
- 各分量**纵向堆叠**，顺序稳定：取组内**最早 created_at**（并列取最小 id）排序。
- 堆叠间距 `GROUP_GAP_Y > NODE_GAP_Y`，让“不同工作流”一眼可辨。

```text
   独立工作流 A（弱连通，一个 DAG）
   独立工作流 B
   ─────  GROUP_GAP_Y(160)  ─────
   独立任务 C（单点）
```

### 第二级 · 单个工作流内部：DAG 布局
给每个分量做**自左向右、分层**的紧凑排版：

1. **分层（决定 X）**：sink 距离法
   - `distance(node) = 0`（无出边依赖）否则 `1 + max(distance(successor))`；
   - 从左到右的列号 `level = maxDistance − distance`（→ 汇合点靠右、起点靠左）。
   - **同层共享一列**，`x = level × (NODE_WIDTH + LEVEL_GAP_X)`。

   ```text
   level 0         level 1         level 2（sink）
   [ A ]──▶[ C ]──▶[ E ]          A: distance=2, level=0
   [ B ]──▶[ D ]──▶[ E ]          E: distance=0, level=2（汇合在右）
   ```

2. **列内排序 / 定 Y（减小交叉 + 对齐邻居）**
   - 初始列序：title 升序、再 id（**稳定**，保证可复现）。
   - 双向 **barycenter（重心）** 扫描 ORDERING_PASSES 轮：每列节点按“邻居列对应节点序号的均值”重新排序，减少边交叉。
   - 迭代 **Y 松弛** POSITIONING_PASSES 轮：
     - 左向扫把每个节点往其**前置的重心**拉，右向扫往**后继的重心**拉；新 Y = 旧 Y 与理想 Y 按 `Y_BLEND=0.5` 混合（防震荡）。
     - 同层相邻边权重 1，跨层边权重 `0.5`（跨层给更弱的垂直提示）。
   - **冲突消解**：列内保证纵向最小间距 `LAYER_SPACING`（正扫向下顶、反扫向上顶，取两者均值 → 不会整体越挤越下）。

> 常量：`NODE_WIDTH=220, NODE_HEIGHT=92, LEVEL_GAP_X=110, NODE_GAP_Y=56, GROUP_GAP_Y=160`。

## 4. 后端画布持久化语义（如果要存用户拖拽）

- 坐标批量 upsert：**last-write-wins**；所有 `task_id` 必须真实存在（否则 Validation/NotFound）。
- 只把用户**手动**给的位置存盘；自动排版是前端一次性渲染的视图，不必入库。
- 手动移动某节点后是否“锁死不自动排”——留给产品在**前端状态**决定（加个 `userPinned` 之类标记即可），后端不为此建字段。

## 5. 搬运取舍表

| 想要的能力 | 要搬什么 | 复杂度 |
|---|---|---|
| 只把 DAG 画得整齐 | `layout.ts` 的分量切割 + 分层 | 低（前后端纯函数） |
| 减少交叉的阅读友好 | barycenter 排序 + Y 松弛 | 中 |
| 用户手动布局并持久化 | `positions` 表 + upsert + 前端拖拽 | 中 |
| 用户自建分组 | `visual_groups` + members | 低 |

> 若新项目规模小，推荐**只搬自动排版**、不搬手动坐标/分组，减少两张表和一个保存链路。
