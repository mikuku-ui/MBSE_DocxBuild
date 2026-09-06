-- 执行：模板在某个执行阶段下摊成执行清单（task 列表化，非 workflow 流式调取）。
-- 清单结构由模板派生；执行人员填的内容（content jsonb）是用户产物，要落库。

CREATE TABLE IF NOT EXISTS execution_instances (
    id          uuid PRIMARY KEY,
    template_id uuid NOT NULL REFERENCES document_templates(id),
    stage       text,             -- 执行阶段（如：文档审查 / 第一轮动态测试 …）
    status      text NOT NULL,    -- draft / in_progress / completed
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_exec_inst_template ON execution_instances (template_id);

CREATE TABLE IF NOT EXISTS execution_items (
    id                uuid PRIMARY KEY,
    instance_id       uuid NOT NULL REFERENCES execution_instances(id) ON DELETE CASCADE,
    template_node_id  uuid NOT NULL REFERENCES document_template_nodes(id),
    title             text NOT NULL,
    status            text NOT NULL,   -- pending / in_progress / completed / skipped
    content           jsonb NOT NULL DEFAULT '{}',
    sort_key          integer NOT NULL DEFAULT 0,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_exec_items_instance ON execution_items (instance_id);
