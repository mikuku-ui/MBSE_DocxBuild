-- 台账重构：拆成「体系（设计，可复用/多套）+ 项目数据（值，项目级一份）」。
--
-- 为什么：台账页把「这类项目要哪些数据（schema）」和「这个项目填了什么（data）」
-- 揉成一张可随意编辑的表。正确切法——schema 上移为「体系」由设计人员设定（可多套，
-- 项目按执行方式选一套绑定）；模板是数据消费者只负责引用；数据在执行清单/项目页填，
-- 落「项目级一份」，装配读它。
--
-- 结构设计：
--   · scheme_* 4 张表 = 设计期定义（字段/表/列）。表列结构允许自定义（列=定义行）。
--   · project_field_values / project_table_rows = 值（项目级一份，不随文档重复）。
--     字段值引用 scheme_fields（表内唯一 (scheme_id, doc_kind, field_key)）；
--     表行引用 scheme_tables，单元格 cells jsonb 以 column_key 为键。
--   · scheme 行按「自然键 upsert 保 id」：改定义不换 id → 已填的项目值不丢；
--     真正被删掉的字段/表级联清其项目值（FK ON DELETE CASCADE）。
--
-- 旧 project_ledger_* 六表本轮保留不删（后端不再引用，后续再出清理迁移）。

-- 默认体系「大纲测评」固定 id（值迁移按它映射）。
-- 00000000-0000-4000-8000-000000000011

CREATE TABLE IF NOT EXISTS schemes (
    id          uuid PRIMARY KEY,
    name        text NOT NULL UNIQUE,
    description text NOT NULL DEFAULT '',
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS scheme_fields (
    id         uuid PRIMARY KEY,
    scheme_id  uuid NOT NULL REFERENCES schemes(id) ON DELETE CASCADE,
    doc_kind   text NOT NULL DEFAULT 'project',
    field_key  text NOT NULL,
    label      text NOT NULL DEFAULT '',
    sort_key   integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (scheme_id, doc_kind, field_key)
);
CREATE INDEX IF NOT EXISTS idx_scheme_fields_scheme ON scheme_fields (scheme_id);

CREATE TABLE IF NOT EXISTS scheme_tables (
    id         uuid PRIMARY KEY,
    scheme_id  uuid NOT NULL REFERENCES schemes(id) ON DELETE CASCADE,
    table_key  text NOT NULL,
    label      text NOT NULL DEFAULT '',
    sort_key   integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (scheme_id, table_key)
);
CREATE INDEX IF NOT EXISTS idx_scheme_tables_scheme ON scheme_tables (scheme_id);

CREATE TABLE IF NOT EXISTS scheme_table_columns (
    id         uuid PRIMARY KEY,
    table_id   uuid NOT NULL REFERENCES scheme_tables(id) ON DELETE CASCADE,
    column_key text NOT NULL,
    label      text NOT NULL DEFAULT '',
    sort_key   integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (table_id, column_key)
);
CREATE INDEX IF NOT EXISTS idx_scheme_cols_table ON scheme_table_columns (table_id);

-- 项目绑体系（可空；空 = 尚未选执行方式）。
ALTER TABLE projects ADD COLUMN IF NOT EXISTS scheme_id uuid;
ALTER TABLE projects ADD CONSTRAINT fk_projects_scheme
    FOREIGN KEY (scheme_id) REFERENCES schemes(id);

-- 项目数据（值）。
CREATE TABLE IF NOT EXISTS project_field_values (
    id         uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    field_id   uuid NOT NULL REFERENCES scheme_fields(id) ON DELETE CASCADE,
    value      text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, field_id)
);
CREATE INDEX IF NOT EXISTS idx_pfv_project ON project_field_values (project_id);

CREATE TABLE IF NOT EXISTS project_table_rows (
    id         uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    table_id   uuid NOT NULL REFERENCES scheme_tables(id) ON DELETE CASCADE,
    row_index  integer NOT NULL DEFAULT 0,
    cells      jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, table_id, row_index)
);
CREATE INDEX IF NOT EXISTS idx_ptr_project ON project_table_rows (project_id);

-- =============================================================================
-- 种子：默认体系「大纲测评」= 现有 13 标量字段 + 5 张表（沿用旧 table source 名）。
-- =============================================================================

INSERT INTO schemes (id, name, description)
VALUES ('00000000-0000-4000-8000-000000000011', '大纲测评',
        '默认体系：标量字段（项目共享 + 大纲专属）+ 依据/参与方/时间地点/被测件/环境 5 张表。')
ON CONFLICT (id) DO NOTHING;

-- 标量字段。doc_kind：project=项目共享；大纲=本册专属。
INSERT INTO scheme_fields (id, scheme_id, doc_kind, field_key, label, sort_key)
SELECT gen_random_uuid(), v.scheme_id, v.doc_kind, v.field_key, v.label, v.sort_key
FROM (VALUES
    ('00000000-0000-4000-8000-000000000011'::uuid, 'project', '项目代号',   '项目代号', 0),
    ('00000000-0000-4000-8000-000000000011'::uuid, 'project', '软件名称',   '软件名称', 1),
    ('00000000-0000-4000-8000-000000000011'::uuid, 'project', '密级',       '密级',     2),
    ('00000000-0000-4000-8000-000000000011'::uuid, 'project', '单位名称',   '单位名称', 3),
    ('00000000-0000-4000-8000-000000000011'::uuid, 'project', '测试性质',   '测试性质', 4),
    ('00000000-0000-4000-8000-000000000011'::uuid, 'project', '测评依据',   '测评依据', 5),
    ('00000000-0000-4000-8000-000000000011'::uuid, '大纲',   '文档标识',   '文档标识', 0),
    ('00000000-0000-4000-8000-000000000011'::uuid, '大纲',   '版本',       '版本',     1),
    ('00000000-0000-4000-8000-000000000011'::uuid, '大纲',   '发布日期',   '发布日期', 2),
    ('00000000-0000-4000-8000-000000000011'::uuid, '大纲',   '拟制',       '拟制',     3),
    ('00000000-0000-4000-8000-000000000011'::uuid, '大纲',   '标审',       '标审',     4),
    ('00000000-0000-4000-8000-000000000011'::uuid, '大纲',   '审核',       '审核',     5),
    ('00000000-0000-4000-8000-000000000011'::uuid, '大纲',   '批准',       '批准',     6)
) AS v(scheme_id, doc_kind, field_key, label, sort_key)
ON CONFLICT (scheme_id, doc_kind, field_key) DO NOTHING;

-- 表 + 列。列 column_key 沿用旧投影的语义键（便于旧数据迁移直接对位）。
INSERT INTO scheme_tables (id, scheme_id, table_key, label, sort_key)
SELECT gen_random_uuid(), v.scheme_id, v.table_key, v.label, v.sort_key
FROM (VALUES
    ('00000000-0000-4000-8000-000000000011'::uuid, 'parties',           '参与方',       0),
    ('00000000-0000-4000-8000-000000000011'::uuid, 'basis_files',       '依据文件',     1),
    ('00000000-0000-4000-8000-000000000011'::uuid, 'test_schedule',     '测评时间和地点', 2),
    ('00000000-0000-4000-8000-000000000011'::uuid, 'devices',           '被测件一览',    3),
    ('00000000-0000-4000-8000-000000000011'::uuid, 'environment_items', '环境软硬件',    4)
) AS v(scheme_id, table_key, label, sort_key)
ON CONFLICT (scheme_id, table_key) DO NOTHING;

-- 列：rows( table_key, column_key, label, sort_key )，列 id 用表内映射子查询取。
INSERT INTO scheme_table_columns (id, table_id, column_key, label, sort_key)
SELECT gen_random_uuid(), t.id, v.column_key, v.label, v.sort_key
FROM (VALUES
    ('parties',           'party_kind',    '角色',   0),
    ('parties',           'name',          '单位名称', 1),
    ('parties',           'address',       '地址',   2),
    ('parties',           'contact',       '联系人', 3),
    ('parties',           'phone',         '电话',   4),
    ('basis_files',       'category',      '类别',   0),
    ('basis_files',       'seq',           '序',     1),
    ('basis_files',       'name',          '文件名称', 2),
    ('basis_files',       'ident',         '文件编号', 3),
    ('basis_files',       'version',       '版本',   4),
    ('basis_files',       'pub_date',      '发布日期', 5),
    ('basis_files',       'pub_org',       '发布单位', 6),
    ('test_schedule',     'seq',           '序',     0),
    ('test_schedule',     'phase',         '阶段',   1),
    ('test_schedule',     'time_range',    '时间',   2),
    ('test_schedule',     'location',      '地点',   3),
    ('devices',           'seq',           '序',     0),
    ('devices',           'name',          '名称',   1),
    ('devices',           'device_type',   '类型',   2),
    ('devices',           'safety_level',  '安全级别', 3),
    ('devices',           'run_env',       '运行环境', 4),
    ('devices',           'dev_env',       '开发环境', 5),
    ('devices',           'language',      '语言',   6),
    ('devices',           'version',       '版本',   7),
    ('devices',           'code_scale',    '代码规模', 8),
    ('devices',           'dev_org',       '开发单位', 9),
    ('environment_items', 'category',      '所属环境', 0),
    ('environment_items', 'resource_kind', '资源类型', 1),
    ('environment_items', 'name',          '名称/型号', 2),
    ('environment_items', 'version_config','版本/配置', 3),
    ('environment_items', 'qty',           '数量',   4),
    ('environment_items', 'note',          '备注',   5),
    ('environment_items', 'provider',      '提供方', 6)
) AS v(table_key, column_key, label, sort_key)
JOIN scheme_tables t ON t.scheme_id = '00000000-0000-4000-8000-000000000011' AND t.table_key = v.table_key
ON CONFLICT (table_id, column_key) DO NOTHING;

-- =============================================================================
-- 迁移：默认项目（00000000-0000-4000-8000-000000000001）旧台账六表 → 项目数据。
-- =============================================================================

-- 标量字段值。
INSERT INTO project_field_values (id, project_id, field_id, value)
SELECT gen_random_uuid(), o.project_id, sf.id, o.value
FROM project_ledger_fields o
JOIN scheme_fields sf
  ON sf.scheme_id = '00000000-0000-4000-8000-000000000011'
 AND sf.doc_kind = o.doc_kind AND sf.field_key = o.field_key
WHERE o.project_id = '00000000-0000-4000-8000-000000000001'
ON CONFLICT (project_id, field_id) DO NOTHING;

-- 参与方行（角色码 → 中文角色名；顺序 client→developer→agency 编 row_index）。
INSERT INTO project_table_rows (id, project_id, table_id, row_index, cells)
SELECT gen_random_uuid(), o.project_id, t.id,
       row_number() OVER (PARTITION BY o.project_id
           ORDER BY CASE o.party_kind WHEN 'client' THEN 1 WHEN 'developer' THEN 2 ELSE 3 END) - 1,
       jsonb_build_object(
           'party_kind',
           CASE o.party_kind
               WHEN 'client' THEN '委托方'
               WHEN 'developer' THEN '研制单位'
               WHEN 'agency' THEN '测评机构'
               ELSE o.party_kind
           END,
           'name', o.name, 'address', o.address,
           'contact', o.contact, 'phone', o.phone)
FROM project_parties o
JOIN scheme_tables t ON t.scheme_id = '00000000-0000-4000-8000-000000000011' AND t.table_key = 'parties'
WHERE o.project_id = '00000000-0000-4000-8000-000000000001'
ON CONFLICT (project_id, table_id, row_index) DO NOTHING;

-- 依据文件行（category,seq 序）。
INSERT INTO project_table_rows (id, project_id, table_id, row_index, cells)
SELECT gen_random_uuid(), o.project_id, t.id,
       row_number() OVER (PARTITION BY o.project_id ORDER BY o.category, o.seq) - 1,
       jsonb_build_object('category', o.category, 'seq', o.seq::text, 'name', o.name,
           'ident', o.ident, 'version', o.version, 'pub_date', o.pub_date, 'pub_org', o.pub_org)
FROM project_basis_files o
JOIN scheme_tables t ON t.scheme_id = '00000000-0000-4000-8000-000000000011' AND t.table_key = 'basis_files'
WHERE o.project_id = '00000000-0000-4000-8000-000000000001'
ON CONFLICT (project_id, table_id, row_index) DO NOTHING;

-- 测评时间地点。
INSERT INTO project_table_rows (id, project_id, table_id, row_index, cells)
SELECT gen_random_uuid(), o.project_id, t.id,
       row_number() OVER (PARTITION BY o.project_id ORDER BY o.seq) - 1,
       jsonb_build_object('seq', o.seq::text, 'phase', o.phase,
           'time_range', o.time_range, 'location', o.location)
FROM project_test_schedule o
JOIN scheme_tables t ON t.scheme_id = '00000000-0000-4000-8000-000000000011' AND t.table_key = 'test_schedule'
WHERE o.project_id = '00000000-0000-4000-8000-000000000001'
ON CONFLICT (project_id, table_id, row_index) DO NOTHING;

-- 被测件一览。
INSERT INTO project_table_rows (id, project_id, table_id, row_index, cells)
SELECT gen_random_uuid(), o.project_id, t.id,
       row_number() OVER (PARTITION BY o.project_id ORDER BY o.seq) - 1,
       jsonb_build_object('seq', o.seq::text, 'name', o.name, 'device_type', o.device_type,
           'safety_level', o.safety_level, 'run_env', o.run_env, 'dev_env', o.dev_env,
           'language', o.language, 'version', o.version, 'code_scale', o.code_scale,
           'dev_org', o.dev_org)
FROM project_devices o
JOIN scheme_tables t ON t.scheme_id = '00000000-0000-4000-8000-000000000011' AND t.table_key = 'devices'
WHERE o.project_id = '00000000-0000-4000-8000-000000000001'
ON CONFLICT (project_id, table_id, row_index) DO NOTHING;

-- 环境软硬件项。
INSERT INTO project_table_rows (id, project_id, table_id, row_index, cells)
SELECT gen_random_uuid(), o.project_id, t.id,
       row_number() OVER (PARTITION BY o.project_id ORDER BY o.category, o.name) - 1,
       jsonb_build_object('category', o.category, 'resource_kind', o.resource_kind,
           'name', o.name, 'version_config', o.version_config, 'qty', o.qty,
           'note', o.note, 'provider', o.provider)
FROM project_environment_items o
JOIN scheme_tables t ON t.scheme_id = '00000000-0000-4000-8000-000000000011' AND t.table_key = 'environment_items'
WHERE o.project_id = '00000000-0000-4000-8000-000000000001'
ON CONFLICT (project_id, table_id, row_index) DO NOTHING;

-- 默认项目绑定该体系。
UPDATE projects
SET scheme_id = '00000000-0000-4000-8000-000000000011'
WHERE id = '00000000-0000-4000-8000-000000000001' AND scheme_id IS NULL;
