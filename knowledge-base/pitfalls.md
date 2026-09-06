# 踩坑清单（Pitfalls）

> 实现这套 workflow+task 过程中**真踩过、修掉、且大概率会在下一项目复现**的坑。
> 分成两组：A. 建模/设计坑（最值得带走的）；B. 数据库/构建/环境坑（本项目踩过的硬教训）。

---

## A · 建模与设计坑

### P1 · 别急着加 parent/children 层级
看到任务就想到“树”是最自然的冲动，但树要求你维护父子一致性、算 depth/顺序、移动子树级联更新。
本项目把任务做成**扁平 + 依赖图**：一切“顺序/层级感”由 `dependency` 图和派生视图给出，结构上根本不存在“半棵树”。

### P2 · 单一事实源：不要“状态 + 平行 completed 布尔”
同时存在 `status` 和 `completed` 就必然出现矛盾态（Todo 但标完成）。完成与否**只从语义枚举派生**，删掉布尔位。

### P3 · 派生量绝不落库
`depth/order/health/execution_state/布局` 若入库，任务/边一改就漂移，还得写一堆“哪里该重算”的代码。
统一“读取期派生”。代价仅是每次投影读全图——数据量小时值得（本项目 `graph()` 每次全量算 health，仍毫秒级）。

### P4 · 环检测只做一半 / 方向用反
- 只在**创建边时**检测并拒绝（`target 是否可达 source`），让“图恒无环”成为不变量；漏了它，拓扑排序排不全、健康递归会栈溢出。
- 注意**只把 dependency 喂进环检测**——reference 永远可成环。
- 方向反了会漏检：新边是 `source→target`，要查的是 **从 target 出发**能否到 source。

### P5 · 一个“执行语义”边类型就够，reference 别到处参与
把“引用/相关”也当依赖去排序、算健康、合并布局，会让无关任务互相阻塞/黏成一张图。
> 规则浓缩成一句：**dependency 负责执行，reference 只负责“看起来相关”。**

### P6 · `status_changed_at` 只在真正换状态时刷新
在普通 edit 也刷新它，会让“上游变动时间”失真 → 健康度 warning 乱报。实现点见 `core` T4。
同理，别拿 `updated_at` 冒充它（updated_at 每次点击都变）。

### P7 · 进度和状态是正交的，别互相触发
进度 100% 自动转完成 / 完成后锁定进度，二者耦合会产生玄学状态（0% 的已完成 / 100% 的未开始）。
要这种联动请在**应用层**做成显式用例，别写进实体构造。

### P8 · 删任务后：关系要么级联删、要么算法容忍“悬挂边”
选择级联删（FK `ON DELETE CASCADE`）最省心；但**任何加载到陈旧关系**的算法（排序/健康/布局）必须跳过“端点已不存在”的边，不能 panic。
> 三种算法对悬挂边一律 `continue` / `parent.has` 先判——这是删除安全的最后防线。

### P9 · 删“被引用 / 某类最后一个”状态要拦
删光某类语义状态（尤其 not_started）会让新任务无默认状态、历史任务无法解释。守卫：被任务引用不可删；某 semantic_kind 仅剩它时不可删。

### P10 · 同名不同物：“自动布局的工作流” vs “用户视觉分组”
自动排版内部按弱连通分量切出的组（临时、不落库）与用户自建视觉分组（持久化容器）是**两回事**，前端千万别混用一套状态，否则拖拽/保存互相覆盖（本项目前后端各一套独立 repo/状态才解耦干净）。

### P11 · 布局算法输入口径必须只有 dependency
弱连通分量、分层、健康、排序……**任何一条**如果混入了 reference 边，就会把两条本无关的工作流粘成一条。

---

## B · 数据库 / 构建 / 环境坑

### P12 · sqlx 迁移文件是“字节级”校验，应用后不可改
sqlx 对每个迁移存 sha384(文件字节)，启动比对。**改已应用过的迁移文件** → checksum 不符直接启动失败。
- 正确姿势：追加新编号迁移；绝不 edit 历史文件。
- 导入旧库时若结构对不上，宁可 `DROP` 掉让新迁移重建，也别手工改迁移。

### P13 · PG 角色 `rolinherit=false`：靠角色成员继承权限 = 静默失效
远程库角色若 `NOINHERIT`，它“是某角色成员”**不等于**能行使该角色权限：`GRANT … ON SCHEMA public TO dev` 静默无效果、`ALTER SCHEMA OWNER` 直接报 “must be member of role”。
- 解法：用成员资格进 `SET ROLE pg_database_owner` → 对 schema 做**直接 ACL** `GRANT USAGE, CREATE … TO dev` → `RESET ROLE`（直接授权绕过 NOINHERIT）。
- 排查口诀：`\du`、`\dn+` 看 ACL 是否真的写给了目标角色。

### P14 · pg_dump 纯 SQL 导入的三连坑（高版本 → PG15）
1. **PG18-only 语法**：`\restrict`/`\unrestrict`、`SET transaction_timeout` 会让 psql 报错——导入前剥离。
2. `SELECT pg_catalog.set_config('search_path','',false)` 会重置 search_path，导致**后续无 schema 限定的校验查询**全部落空——剥离或手动重设。
3. **COPY 表序未必满足 FK**：本 dump `workflow_visual_group_members` 排在 `workflow_visual_groups` **之前**，直接按文件顺序导会 FK 冲突。导数据按 **groups → members → positions** 的 FK 序手动喂。
4. 用 psql 导含中文数据：设 `PGCLIENTENCODING=UTF8`，否则 Windows 下中文标题乱码。

### P15 · Windows 下 psql 不在 PATH、且要密码
本项目机器：`C:\Users\24171\.dbclient\dependency\postgresql\psql.exe`（无 pg_dump/restore），连接需 `PGPASSWORD=...`。
- 结论：**交互式 DB 操作全走 psql，别为一次性导入再造 Python/psycopg 脚本**（后者还会引出“污染 conda 环境”的连锁问题，见 P17）。

### P16 · 前端“改完没用”≠ 代码错：进程没换
后端/前端热更时，旧进程占着端口没死，新请求打到旧二进制（路由 404 / 旧逻辑）。
- 症状参考：加了 `/api/workflow/layout` 后仍报“保存失败”→ 多半是旧进程。
- 清端口用 `taskkill //PID <pid> //T //F`（`//T` 连子进程一起杀，否则子进程继续占端口）。

### P17 · 别把一次性工具的依赖装进长期环境
用隔离 venv 跑一次性还原/校验脚本，跑完**连 venv 一起删**；别 pip 进 conda base / 命名环境。
（本仓库教训：为还原临时建的 `SQL/.venv` + psycopg 脚本，事后按用户要求整体清除，改用 psql。）

---

## 带走的排查速查
| 现象 | 先查 |
|---|---|
| 迁移启动失败 checksum | P12 是否改过已应用迁移 |
| 权限明明 grant 了还 denied | P13 NOINHERIT / ACL 没写对角色 |
| 导入报 COPY/FK 错 | P14 表序 + search_path + PG18 语法 |
| 中文乱码 | P14-4 编码 |
| 改完功能没生效 / 保存失败 | P16 旧进程占端口 |
| 无关任务互相阻塞 / 画布黏一团 | P5 / P11 reference 混进图 |
