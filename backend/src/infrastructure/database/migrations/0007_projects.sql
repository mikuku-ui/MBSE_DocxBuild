-- 项目（Project）维度：追踪图按项目隔离，每个项目一套独立的追踪数据。
--
-- 范围仅覆盖需求追踪（trace_nodes / trace_links / trace_baselines）；
-- 文档模板 / 执行清单维持全局不变（与 0001–0006 唯一键语义一致）。
--
-- 固定「默认示例项目」id（迁移回填存量追踪数据 + 前端首个可用的项目）：
--   00000000-0000-4000-8000-000000000001，名「默认示例项目」。
-- 唯一键由「全局 external 键唯一」收紧为「项目内 external 键唯一」——
-- 跨项目允许出现相同 external_source + external_ref（各自回链大系统同一对象）。
CREATE TABLE IF NOT EXISTS projects (
    id         uuid PRIMARY KEY,
    name       text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO projects (id, name)
VALUES ('00000000-0000-4000-8000-000000000001', '默认示例项目')
ON CONFLICT (id) DO NOTHING;

-- trace_nodes：加 project_id，存量回填默认项目，唯一键改为「项目内唯一」。
ALTER TABLE trace_nodes ADD COLUMN IF NOT EXISTS project_id uuid;
UPDATE trace_nodes
SET project_id = '00000000-0000-4000-8000-000000000001'
WHERE project_id IS NULL;
ALTER TABLE trace_nodes ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE trace_nodes ADD CONSTRAINT fk_trace_nodes_project
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE;
ALTER TABLE trace_nodes DROP CONSTRAINT IF EXISTS trace_nodes_external_source_external_ref_key;
ALTER TABLE trace_nodes ADD CONSTRAINT uq_trace_nodes_project_external
    UNIQUE (project_id, external_source, external_ref);
CREATE INDEX IF NOT EXISTS idx_trace_nodes_project ON trace_nodes (project_id);

-- trace_links：project_id 从 source 节点继承；唯一键保持不变（节点 id 全局唯一）。
ALTER TABLE trace_links ADD COLUMN IF NOT EXISTS project_id uuid;
UPDATE trace_links
SET project_id = (
    SELECT n.project_id
    FROM trace_nodes n
    WHERE n.id = trace_links.source_node_id
    LIMIT 1
)
WHERE project_id IS NULL;
ALTER TABLE trace_links ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE trace_links ADD CONSTRAINT fk_trace_links_project
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_trace_links_project ON trace_links (project_id);

-- trace_baselines：一并项目隔离（低风险，前端暂未展示）。
ALTER TABLE trace_baselines ADD COLUMN IF NOT EXISTS project_id uuid;
UPDATE trace_baselines
SET project_id = '00000000-0000-4000-8000-000000000001'
WHERE project_id IS NULL;
ALTER TABLE trace_baselines ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE trace_baselines ADD CONSTRAINT fk_trace_baselines_project
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_trace_baselines_project ON trace_baselines (project_id);
