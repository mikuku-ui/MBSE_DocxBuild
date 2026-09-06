-- 追踪链接：有向 + 类型化 + 单一规范方向。
--
-- 方向约定钉死：source = 下级/派生/验证方，target = 上级/被满足方
-- （即 测试需求 --verifies--> 软件需求、测试需求 --derives_from--> 软件需求）。
-- link_type: derives_from / verifies / satisfies / refines / part_of
CREATE TABLE IF NOT EXISTS trace_links (
    id             uuid PRIMARY KEY,
    source_node_id uuid NOT NULL REFERENCES trace_nodes(id) ON DELETE CASCADE,
    target_node_id uuid NOT NULL REFERENCES trace_nodes(id) ON DELETE CASCADE,
    link_type      text NOT NULL,
    attributes     jsonb NOT NULL DEFAULT '{}',
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE (source_node_id, target_node_id, link_type),
    CHECK (source_node_id <> target_node_id)
);

CREATE INDEX IF NOT EXISTS idx_trace_links_source ON trace_links (source_node_id);
CREATE INDEX IF NOT EXISTS idx_trace_links_target ON trace_links (target_node_id);
