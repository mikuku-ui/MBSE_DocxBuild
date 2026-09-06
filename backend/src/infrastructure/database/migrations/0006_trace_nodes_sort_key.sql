-- 节点在所属阶段（泳道）内的排序序号。用户自由拖拽 / 左栏上移下移 / 「整理」按钮
-- 都会写入 sort_key（阶段内 0..n-1 序数，非全局唯一）；画布纵向排布与左栏列表
-- 都按 (sort_key, 自然序 ref) 展示，二者同源。摄取/幂等 upsert 时不覆盖已存在的
-- sort_key（保留用户手动顺序）；新插入节点默认取 0，由后端按其阶段末尾序号写入。
ALTER TABLE trace_nodes
    ADD COLUMN IF NOT EXISTS sort_key integer NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_trace_nodes_kind_sort ON trace_nodes (kind, sort_key);
