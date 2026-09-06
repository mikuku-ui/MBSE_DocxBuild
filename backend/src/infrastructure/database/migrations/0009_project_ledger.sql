-- 项目台账（Project Ledger）：文档装配的「燃料」。
--
-- 定位（对齐知识库「文档只是视图，不存业务数据」）：
--   · 文档模板节点用 content_spec 声明「这里的内容从哪来」；
--   · 装配引擎在渲染/预览时从「追踪图 + 项目台账」把正文算出来；
--   · 台账 = 一个项目在测评周期里填写的基础信息，按被消费的最小切片分表存放，
--     贴近参考 `项目文档基础信息V1.2.xlsx` 的栏目（委托方/依据文件/测评时间地点/
--     被测件/环境软硬件项）。
--
-- 约定：
--   · doc_kind：标量字段归属哪个模板作用域。'project' = 项目共享（软件名称/项目代号/
--     密级…）；其余 = 具体文档类（大纲/说明/记录/问题报告/报告）的专属字段（文档标识/
--     版本/拟制/审核/批准…）。模板装配时缺省取模板 kind，找不到回退 'project'。
--   · 均为 text 列（版本/日期等自由书写），不做 DB 级 CHECK——体系会演化，约束放
--     应用层校验语义，这里只保证引用完整与幂等。
--   · 表行 id 由应用层生成（uuid PK）；自然唯一键做幂等种子/防重。

CREATE TABLE IF NOT EXISTS project_ledger_fields (
    id         uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    doc_kind   text NOT NULL DEFAULT 'project',
    field_key  text NOT NULL,
    value      text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, doc_kind, field_key)
);
CREATE INDEX IF NOT EXISTS idx_ledger_fields_project ON project_ledger_fields (project_id);

-- 参与方：委托方 / 研制单位 / 测评机构。party_kind ∈ client | developer | agency。
-- 每个项目每类参与方至多一条（参考大纲里三张 4×2 的单行信息表）。
CREATE TABLE IF NOT EXISTS project_parties (
    id         uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    party_kind text NOT NULL,
    name       text NOT NULL,
    address    text NOT NULL DEFAULT '',
    contact    text NOT NULL DEFAULT '',
    phone      text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, party_kind)
);
CREATE INDEX IF NOT EXISTS idx_parties_project ON project_parties (project_id);

-- 依据文件。category ∈ 管理类文件 | 顶层技术文件 | 被测软件文件 | 其他文件。
CREATE TABLE IF NOT EXISTS project_basis_files (
    id         uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    category   text NOT NULL,
    seq        integer NOT NULL DEFAULT 0,
    name       text NOT NULL,
    ident      text NOT NULL DEFAULT '',
    version    text NOT NULL DEFAULT '',
    pub_date   text NOT NULL DEFAULT '',
    pub_org    text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_basis_project ON project_basis_files (project_id);

-- 测评时间地点（大纲「测评时间和地点」表）。
CREATE TABLE IF NOT EXISTS project_test_schedule (
    id         uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    seq        integer NOT NULL DEFAULT 0,
    phase      text NOT NULL,
    time_range text NOT NULL DEFAULT '',
    location   text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_schedule_project ON project_test_schedule (project_id);

-- 被测件一览（配置项级清单：被测软件/硬件 + 研制信息）。
CREATE TABLE IF NOT EXISTS project_devices (
    id           uuid PRIMARY KEY,
    project_id   uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    seq          integer NOT NULL DEFAULT 0,
    name         text NOT NULL,
    device_type  text NOT NULL DEFAULT '',   -- 配置项 / 软件配置项 / 系统 …
    safety_level text NOT NULL DEFAULT '',
    run_env      text NOT NULL DEFAULT '',
    dev_env      text NOT NULL DEFAULT '',
    language     text NOT NULL DEFAULT '',
    version      text NOT NULL DEFAULT '',
    code_scale   text NOT NULL DEFAULT '',
    dev_org      text NOT NULL DEFAULT '',
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_devices_project ON project_devices (project_id);

-- 环境软硬件项（大纲「测评环境所使用的软硬件项」表）。category = 所属环境名
-- （静态测评环境 / AAA动态测评环境 …），由应用层维护枚举语义。
CREATE TABLE IF NOT EXISTS project_environment_items (
    id             uuid PRIMARY KEY,
    project_id     uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    category       text NOT NULL,
    resource_kind  text NOT NULL DEFAULT '',   -- 硬件资源 | 软件资源
    name           text NOT NULL,
    version_config text NOT NULL DEFAULT '',
    qty            text NOT NULL DEFAULT '',
    note           text NOT NULL DEFAULT '',
    provider       text NOT NULL DEFAULT '',
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_env_items_project ON project_environment_items (project_id);

-- =============================================================================
-- 种子：默认示例项目（00000000-0000-4000-8000-000000000001）一套演示台账，
-- 参考 参考/【RTxx0xx0xx】项目文档基础信息V1.2.xlsx + 大纲样张栏目。全部幂等。
-- =============================================================================

-- 标量字段：项目共享（project）+ 大纲专属（大纲）。
INSERT INTO project_ledger_fields (id, project_id, doc_kind, field_key, value)
SELECT gen_random_uuid(), v.project_id, v.doc_kind, v.field_key, v.value
FROM (VALUES
    ('00000000-0000-4000-8000-000000000001'::uuid, 'project', '项目代号',     'RT25046001'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'project', '软件名称',     'XXX系统（或软件）'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'project', '密级',         '秘密★10年'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'project', '单位名称',     '西安润道智检科技有限公司'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'project', '测试性质',     '软件鉴定测评'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'project', '测评依据',     '依据XX发布的XX号《XX软件鉴定测评总体方案》'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '大纲',   '文档标识',     'RT25046001-TF-FTC'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '大纲',   '版本',         'V1.0'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '大纲',   '发布日期',     '2025.06'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '大纲',   '拟制',         '张三'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '大纲',   '标审',         '李四'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '大纲',   '审核',         '王五'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '大纲',   '批准',         '赵六')
) AS v(project_id, doc_kind, field_key, value)
ON CONFLICT (project_id, doc_kind, field_key) DO NOTHING;

-- 参与方（三类各一）。
INSERT INTO project_parties (id, project_id, party_kind, name, address, contact, phone)
SELECT gen_random_uuid(), v.project_id, v.party_kind, v.name, v.address, v.contact, v.phone
FROM (VALUES
    ('00000000-0000-4000-8000-000000000001'::uuid, 'client',    'XXX试验监管局',     'XX市XX路XX号',      'XX', '01066XXXXX'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'developer', 'XXX（研制单位）',   'XX市XX路XX号',      'XX', '010XXXXXX'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'agency',    '西安润道智检科技有限公司', '西安市XX区XX路XX号', 'XX', '029XXXXXXX')
) AS v(project_id, party_kind, name, address, contact, phone)
ON CONFLICT (project_id, party_kind) DO NOTHING;

-- 依据文件（项目为空时才灌）。
INSERT INTO project_basis_files (id, project_id, category, seq, name, ident, version, pub_date, pub_org)
SELECT gen_random_uuid(), v.project_id, v.category, v.seq, v.name, v.ident, v.version, v.pub_date, v.pub_org
FROM (VALUES
    ('00000000-0000-4000-8000-000000000001'::uuid, '管理类文件',   1, 'XX软件鉴定测评总体方案', 'XX[2025]XX号', 'V1.0',   '2025.01', 'XX机关'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '管理类文件',   2, '武器装备软件测评管理要求', 'GJB 9001C', '——',   '2020.05', 'XX标准化'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '顶层技术文件', 1, 'XX系统（或软件）需求规格说明', 'XX-RS-001',  'V2.00.00', '2025.01', 'XX（研制单位）'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '顶层技术文件', 2, 'XX系统（或软件）软件设计说明', 'XX-DD-001',  'V2.01.00', '2025.03', 'XX（研制单位）'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '被测软件文件', 1, 'XX软件源代码及编译说明', 'SRC-001', 'V2.00.00', '2025.04', 'XX（研制单位）'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '其他文件',     1, 'XXX软件鉴定测评大纲（本文件）', 'RT25046001-TF-FTC', 'V1.0', '2025.06', '西安润道智检科技有限公司')
) AS v(project_id, category, seq, name, ident, version, pub_date, pub_org)
WHERE NOT EXISTS (SELECT 1 FROM project_basis_files WHERE project_id = '00000000-0000-4000-8000-000000000001');

-- 测评时间地点（项目为空时才灌）。
INSERT INTO project_test_schedule (id, project_id, seq, phase, time_range, location)
SELECT gen_random_uuid(), v.project_id, v.seq, v.phase, v.time_range, v.location
FROM (VALUES
    ('00000000-0000-4000-8000-000000000001'::uuid, 1, '测评总体方案编制', '2025.01-2025.02', 'XX省XX市XX单位'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 2, '测评大纲编制',     '2025.02-2025.06', 'XX省XX市XX单位'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 3, '测评实施（静态+动态）', '2025.06-2025.08', 'XX省XX市XX测评机构'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 4, '测试问题回归',     '2025.08-2025.09', 'XX省XX市XX测评机构'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 5, '测试报告编写',     '2025.09-2025.10', 'XX省XX市XX测评机构')
) AS v(project_id, seq, phase, time_range, location)
WHERE NOT EXISTS (SELECT 1 FROM project_test_schedule WHERE project_id = '00000000-0000-4000-8000-000000000001');

-- 被测件一览（项目为空时才灌）。
INSERT INTO project_devices (id, project_id, seq, name, device_type, safety_level, run_env, dev_env, language, version, code_scale, dev_org)
SELECT gen_random_uuid(), v.project_id, v.seq, v.name, v.device_type, v.safety_level, v.run_env, v.dev_env, v.language, v.version, v.code_scale, v.dev_org
FROM (VALUES
    ('00000000-0000-4000-8000-000000000001'::uuid, 1, 'XXX软件',  '软件配置项', 'B', '中标麒麟V7.0', 'XX', 'C/C++', 'V2.00.00', '约XX万行', 'XX（研制单位）'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 2, 'YYY软件',  '软件配置项', 'B', '中标麒麟V7.0', 'XX', 'Ada',    'V1.30.00', '约XX万行', 'XX（研制单位）'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 3, 'ZZZ设备',  '设备',       '——', '——',           '——', '固件',  'V1.00',    '——',     'XX（研制单位）')
) AS v(project_id, seq, name, device_type, safety_level, run_env, dev_env, language, version, code_scale, dev_org)
WHERE NOT EXISTS (SELECT 1 FROM project_devices WHERE project_id = '00000000-0000-4000-8000-000000000001');

-- 环境软硬件项（项目为空时才灌）。
INSERT INTO project_environment_items (id, project_id, category, resource_kind, name, version_config, qty, note, provider)
SELECT gen_random_uuid(), v.project_id, v.category, v.resource_kind, v.name, v.version_config, v.qty, v.note, v.provider
FROM (VALUES
    ('00000000-0000-4000-8000-000000000001'::uuid, '静态测评环境',      '硬件资源', 'XXX工作站', '处理器：xxx / 内存：xxxGB', '2', '代码审查、静态分析用', '测评机构'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '静态测评环境',      '软件资源', '代码审查工具', '版本Vx.x',             '2', '——',                 '测评机构'),
    ('00000000-0000-4000-8000-000000000001'::uuid, '静态测评环境',      '软件资源', '静态分析工具', '版本Vx.x',             '2', '——',                 '测评机构'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'AAA动态测评环境',   '硬件资源', 'XXX设备',     '设备标识：xxx',          '1', '产生XXX激励数据',      '测评机构'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'AAA动态测评环境',   '硬件资源', 'YYY设备',     '设备标识：xxx',          '1', '——',                 '测评机构'),
    ('00000000-0000-4000-8000-000000000001'::uuid, 'AAA动态测评环境',   '软件资源', '测试工具软件', '版本Vx.x',             '1', '驻留XX设备，产生激励', '测评机构')
) AS v(project_id, category, resource_kind, name, version_config, qty, note, provider)
WHERE NOT EXISTS (SELECT 1 FROM project_environment_items WHERE project_id = '00000000-0000-4000-8000-000000000001');
