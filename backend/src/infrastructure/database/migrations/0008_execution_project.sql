-- 执行清单项目化：执行实例归属项目（大模块执行依据 = 项目）。
--
-- 需求追踪已按项目隔离（0007）；本轮把执行清单也对齐到项目维度——执行清单的
-- 工作流是「创建项目 → 为项目选文档模板 → 按模板生成执行实例」，因此实例要记
-- 归属项目。文档模板仍是全局通用资源（不分项目），这里不改。
--
-- execution_instances 加 project_id，存量回填默认示例项目；execution_items 不加
-- project_id——它经 instance_id 归属，删除项目级联清空实例即可连带清空。
ALTER TABLE execution_instances ADD COLUMN IF NOT EXISTS project_id uuid;

UPDATE execution_instances
SET project_id = '00000000-0000-4000-8000-000000000001'
WHERE project_id IS NULL;

ALTER TABLE execution_instances ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE execution_instances ADD CONSTRAINT fk_exec_inst_project
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_exec_inst_project ON execution_instances (project_id);
