// 与后端 serde JSON 契约一一对应的 TS 类型（snake_case 原样保留）。

// ---------- project ----------
// 追踪图按项目隔离（0007 迁移）；每个项目一套独立追踪数据。项目还绑定一套「体系」
// （= 执行方式，见下方 scheme 段）：体系定义「这类项目要哪些数据」，项目数据落库的值
// 引用它。项目范围仅覆盖需求追踪 / 执行清单 / 项目数据，文档模板仍全局通用。
export interface Project {
  id: string;
  name: string;
  /** 绑定的体系 id（空 = 尚未选执行方式，项目数据待绑定后才有结构）。 */
  scheme_id: string | null;
  created_at: string;
  updated_at: string;
}

// ---------- trace ----------
// 需求测试五阶段：最左 = 测试报告（最低层），最右 = 需求（顶层）。
// 左侧阶段的产物用于覆盖/回答其右侧阶段。
export type TraceNodeKind =
  | 'software_requirement' // 需求（顶层/最右）
  | 'test_item' // 测试项
  | 'test_case' // 测试用例
  | 'test_record' // 测试记录
  | 'test_report'; // 测试报告（最低层/最左）

export type LinkType = 'derives_from' | 'verifies' | 'satisfies' | 'refines' | 'part_of';

export interface TraceNode {
  id: string;
  /** 归属项目（追踪数据按项目隔离） */
  project_id: string;
  kind: TraceNodeKind;
  external_source: string;
  external_ref: string;
  module: string | null;
  title: string;
  description: string | null;
  attributes: Record<string, unknown>;
  /** 用户在所属阶段（泳道）内自定的排序序号，画布纵向与左栏列表同源；落库保存 */
  sort_key: number;
  created_at: string;
  updated_at: string;
}

export interface TraceLink {
  id: string;
  /** 归属项目（两端节点必然同项目） */
  project_id: string;
  source_node_id: string;
  target_node_id: string;
  link_type: LinkType;
  attributes: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface TraceBaseline {
  id: string;
  project_id: string;
  name: string;
  reason: string | null;
  snapshot_json: unknown;
  created_at: string;
}

export interface TraceMatrixRow {
  requirement: TraceNode;
  covering: Array<[TraceNode, LinkType]>;
}

export interface TraceMatrix {
  rows: TraceMatrixRow[];
}

// ---------- document ----------
export type TemplateNodeType =
  | 'frontmatter'
  | 'chapter'
  | 'section'
  | 'paragraph'
  | 'table'
  | 'figure'
  | 'appendix'
  | 'trace_view';

export interface DocumentTemplate {
  id: string;
  name: string;
  kind: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface TemplateNode {
  id: string;
  template_id: string;
  node_type: TemplateNodeType;
  title: string;
  content_spec: Record<string, unknown>;
  tex_component: string | null;
  sort_key: number;
  created_at: string;
  updated_at: string;
}

export interface TemplateEdge {
  id: string;
  template_id: string;
  source_node_id: string;
  target_node_id: string;
  edge_type: string;
  created_at: string;
}

export interface TemplateFull {
  template: DocumentTemplate;
  nodes: TemplateNode[];
  edges: TemplateEdge[];
}

// ---------- execution ----------
export type InstanceStatus = 'draft' | 'in_progress' | 'completed';

export type ItemStatus = 'pending' | 'in_progress' | 'completed' | 'skipped';

export interface ExecutionInstance {
  id: string;
  /** 归属项目（执行清单按项目隔离，0008 迁移）；模板本身全局通用 */
  project_id: string;
  template_id: string;
  stage: string | null;
  status: InstanceStatus;
  created_at: string;
  updated_at: string;
}

export interface ExecutionItem {
  id: string;
  instance_id: string;
  template_node_id: string;
  title: string;
  status: ItemStatus;
  content: Record<string, unknown>;
  sort_key: number;
  created_at: string;
  updated_at: string;
}

export interface ExecutionDetail {
  instance: ExecutionInstance;
  items: ExecutionItem[];
}

// ---------- 通用展示映射 ----------
export const NODE_KIND_LABEL: Record<TraceNodeKind, string> = {
  software_requirement: '需求',
  test_item: '测试项',
  test_case: '测试用例',
  test_record: '测试记录',
  test_report: '测试报告',
};

// 阶段在流水上的说明（自上而下/自右向左）：左侧的一切用于覆盖/回答右侧。
export const STAGE_SUBLINE: Record<TraceNodeKind, string> = {
  software_requirement: '被测顶层（研发侧或文档录入）',
  test_item: '设计出来以满足需求',
  test_case: '由测试项分析出的用例',
  test_record: '对用例执行后获得的结果',
  test_report: '据记录回答需求是否真被满足',
};

export const LINK_TYPE_LABEL: Record<LinkType, string> = {
  derives_from: '派生自',
  verifies: '验证',
  satisfies: '满足',
  refines: '细化',
  part_of: '包含',
};

export const NODE_TYPE_LABEL: Record<TemplateNodeType, string> = {
  frontmatter: '封面/前置',
  chapter: '章',
  section: '节',
  paragraph: '段落',
  table: '表格',
  figure: '图',
  appendix: '附录',
  trace_view: '追溯视图',
};

export const INSTANCE_STATUS_LABEL: Record<InstanceStatus, string> = {
  draft: '草稿',
  in_progress: '进行中',
  completed: '已完成',
};

export const ITEM_STATUS_LABEL: Record<ItemStatus, string> = {
  pending: '待办',
  in_progress: '进行中',
  completed: '已完成',
  skipped: '跳过',
};

// ---------- 体系（台账编排 / 项目执行方式）----------
// 体系 = 项目数据结构的「设计期定义」：标量字段 + 表（表结构 = 自定义列）。可复用、可多套；
// 项目按执行方式绑定一套（projects.scheme_id）。模板是数据消费者，只引用 field_key/table_key。
// GET /api/schemes → 元数据清单；GET /api/schemes/{id} → SchemeFull（含 id 的完整定义）。

export interface Scheme {
  id: string;
  name: string;
  description: string;
  created_at: string;
  updated_at: string;
}

export interface SchemeField {
  id: string;
  scheme_id: string;
  /** 'project' = 项目共享；其余（大纲/说明/记录/问题报告/报告）= 该文档类专属作用域。 */
  doc_kind: string;
  field_key: string;
  label: string;
  sort_key: number;
  created_at: string;
  updated_at: string;
}

export interface SchemeTable {
  id: string;
  scheme_id: string;
  table_key: string;
  label: string;
  sort_key: number;
  created_at: string;
  updated_at: string;
}

export interface SchemeTableColumn {
  id: string;
  table_id: string;
  column_key: string;
  label: string;
  sort_key: number;
  created_at: string;
  updated_at: string;
}

/** 一张表 + 其列（完整定义里的嵌套单元）。 */
export interface SchemeTableDef {
  table: SchemeTable;
  columns: SchemeTableColumn[];
}

/** 体系的完整定义（含 id）：设计页编辑 / 装配取结构用。 */
export interface SchemeFull {
  scheme: Scheme;
  fields: SchemeField[];
  tables: SchemeTableDef[];
}

// 写入形态（POST/PUT /api/schemes 请求体）：只含业务列，无 id/时间戳/sort_key。
// 服务端按自然键 upsert（保 id → 已填项目值不丢），sort_key 按数组序重编。
export interface SchemeFieldSpec {
  doc_kind?: string;
  field_key: string;
  label?: string;
}

export interface SchemeColumnSpec {
  column_key: string;
  label?: string;
}

export interface SchemeTableSpec {
  table_key: string;
  label?: string;
  columns?: SchemeColumnSpec[];
}

export interface SchemeSpec {
  name: string;
  description?: string;
  fields?: SchemeFieldSpec[];
  tables?: SchemeTableSpec[];
}

/** 字段的文档作用域可选值（体系设计 / 模板字段声明共用下拉）。 */
export const DOC_KIND_OPTIONS = [
  'project',
  '大纲',
  '说明',
  '记录',
  '问题报告',
  '报告',
] as const;

// ---------- 项目数据（值；项目级一份，全项目文档共享）----------
// 结构来自项目绑定体系；本段类型即 GET/PUT /api/projects/{id}/data 的契约
// （数据视图 = 结构 × 值，自描述可直接渲染）。值落库一次，装配/项目页/执行清单共读共写。
export interface DataField {
  id: string;
  doc_kind: string;
  field_key: string;
  label: string;
  value: string;
}

export interface DataColumn {
  id: string;
  column_key: string;
  label: string;
}

export interface DataRow {
  row_index: number;
  /** cells 以 column_key 为键；缺键 = 空单元格。 */
  cells: Record<string, string>;
}

export interface DataTable {
  id: string;
  table_key: string;
  label: string;
  columns: DataColumn[];
  rows: DataRow[];
}

/** 项目数据视图：GET 响应形态 / 装配取数源。scheme_id=null = 项目未绑体系。 */
export interface ProjectDataView {
  project_id: string;
  scheme_id: string | null;
  fields: DataField[];
  tables: DataTable[];
}

/** PUT /api/projects/{id}/data 请求体：整存替换，引用体系 field/table 的 id。 */
export interface ProjectDataWrite {
  fields: Array<{ field_id: string; value: string }>;
  tables: Array<{ table_id: string; rows: DataRow[] }>;
}

// ---------- 内容装配（五原语）----------
// 镜像后端 `application/assembly/content.rs` 的 ContentSpec（jsonb 判别结构）。
export type ContentSpec =
  | { kind: 'text'; text: string }
  | { kind: 'field'; field: string; doc_kind?: string | null }
  | {
      kind: 'table';
      source: string;
      party_kind?: string | null;
      category?: string | null;
    }
  | { kind: 'trace'; view: string }
  | { kind: 'slot'; placeholder: string; role?: string | null };

/** 装配块的 kind：无/未知 content_spec → 'empty'（纯结构节点）。 */
export type BlockKind = 'text' | 'field' | 'table' | 'trace' | 'slot' | 'empty';

export const BLOCK_KIND_LABEL: Record<BlockKind, string> = {
  text: '文本套话',
  field: '字段',
  table: '数据表',
  trace: '追溯切片',
  slot: '人工槽',
  empty: '（无内容）',
};

export const TRACE_VIEW_LABEL: Record<string, string> = {
  matrix: '需求→测试项追溯矩阵',
  test_items: '测试项清单',
};

export interface ResolvedBlock {
  node_id: string;
  node_type: TemplateNodeType;
  title: string;
  /** 文档层级（contains 树深度；根 = 0）。 */
  depth: number;
  kind: BlockKind;
  /**
   * 按 kind 有结构的值：
   * - text/field → { text }
   * - table       → { columns: [{ key, label }], rows: [ {<column_key>: 值} ] }
   * - trace       → { view, rows|items }
   * - slot        → { placeholder, role, filled, text }
   * - empty       → {}
   */
  value: Record<string, unknown>;
}

export interface ResolvedDocument {
  template_id: string;
  template_name: string;
  template_kind: string;
  project_id: string;
  instance_id: string | null;
  blocks: ResolvedBlock[];
}
