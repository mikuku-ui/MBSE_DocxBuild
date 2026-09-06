-- 文档模板（Document）：与需求追踪图是两张彼此独立的图。
-- 模板节点 = 文档结构元素（章/节/段落/表格/图/附录/追踪章节…），串起来 = 一篇模板。
-- 二者只在渲染期桥接（trace_view 节点派生读取追踪图），无 FK 强耦合。

CREATE TABLE IF NOT EXISTS document_templates (
    id          uuid PRIMARY KEY,
    name        text NOT NULL,
    kind        text NOT NULL,       -- 大纲 / 报告 / …（对应 ConTeXt kind）
    description text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS document_template_nodes (
    id            uuid PRIMARY KEY,
    template_id   uuid NOT NULL REFERENCES document_templates(id) ON DELETE CASCADE,
    node_type     text NOT NULL,     -- frontmatter/chapter/section/paragraph/table/figure/appendix/trace_view
    title         text NOT NULL,
    content_spec  jsonb NOT NULL DEFAULT '{}',   -- 该处要什么内容；trace_view 存追踪派生配置
    tex_component text,              -- tex 预留字段（生成器暂不实现，只占位钉死接口）
    sort_key      integer NOT NULL DEFAULT 0,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_dtn_template ON document_template_nodes (template_id);

-- 节点互挂边（表结构顺序 / 包含关系）。不做 parent_id 硬编码列。
CREATE TABLE IF NOT EXISTS document_template_edges (
    id             uuid PRIMARY KEY,
    template_id    uuid NOT NULL REFERENCES document_templates(id) ON DELETE CASCADE,
    source_node_id uuid NOT NULL REFERENCES document_template_nodes(id) ON DELETE CASCADE,
    target_node_id uuid NOT NULL REFERENCES document_template_nodes(id) ON DELETE CASCADE,
    edge_type      text NOT NULL DEFAULT 'contains',
    created_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE (template_id, source_node_id, target_node_id, edge_type),
    CHECK (source_node_id <> target_node_id)
);
CREATE INDEX IF NOT EXISTS idx_dte_template ON document_template_edges (template_id);
