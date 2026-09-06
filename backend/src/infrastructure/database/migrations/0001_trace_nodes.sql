-- 需求追踪图节点：大 MBSE 系统灌入本子系统的「数据入口」的干净投影。
--
-- external_source + external_ref = 回链大系统的幂等键（UNIQUE），ingest 用 ON
-- CONFLICT 做幂等 upsert；module 为软分组（规格说明/集合），追溯矩阵据此分组；
-- attributes 透传大系统持有的类型化属性，schema 由大系统拥有，本系统只透传。
CREATE TABLE IF NOT EXISTS trace_nodes (
    id              uuid PRIMARY KEY,
    kind            text NOT NULL,                       -- software_requirement/test_requirement/test_case
    external_source text NOT NULL,
    external_ref    text NOT NULL,
    module          text,
    title           text NOT NULL,
    description     text,
    attributes      jsonb NOT NULL DEFAULT '{}',
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE (external_source, external_ref)
);

CREATE INDEX IF NOT EXISTS idx_trace_nodes_kind ON trace_nodes (kind);
CREATE INDEX IF NOT EXISTS idx_trace_nodes_module ON trace_nodes (module);
