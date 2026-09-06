//! 五原语内容契约（存于模板节点 `content_spec` jsonb）+ 解析。
//!
//! 每种 kind 声明「这一处的内容从哪来」：
//! - `text`   套话/正文，可含 `{field_key}` 占位符（装配时用项目字段值展开）；
//! - `field`  项目数据标量字段（`{field_key}`，可带 doc_kind 作用域，缺省跟随文档类）；
//! - `table`  项目数据表：source = 项目绑定体系里的 **table_key**（旧 parties/
//!            basis_files/… 即默认体系的表键）；可带可选过滤列 party_kind / category
//!            ——按同名列单元格精确匹配（默认体系 parties 存中文角色、basis_files/
//!            environment_items 存类别，均可用此列过滤）；
//! - `trace`  追溯切片：view = matrix（需求→测试项追溯矩阵）| test_items（测试项清单）；
//! - `slot`   人工槽：模板留空、由执行人员在执行清单中填写（placeholder 兜底）。
//!
//! 体系加字段/表列 = 项目页给绑定体系补结构 + 模板放个 `field`/`table` 节点 →
//! 引擎零改动（「体系会演化」）。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 装配块的内容类别（与 `ContentSpec.kind` 同构，另加 `empty` 表达纯结构/未声明内容）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Text,
    Field,
    Table,
    Trace,
    Slot,
    Empty,
}

impl BlockKind {
    /// 从 `ContentSpec` 的 kind 字段映射；无/未知 → `Empty`。
    pub fn from_spec(value: &Value) -> Self {
        let kind = value.get("kind").and_then(Value::as_str).unwrap_or("");
        match kind {
            "text" => Self::Text,
            "field" => Self::Field,
            "table" => Self::Table,
            "trace" => Self::Trace,
            "slot" => Self::Slot,
            _ => Self::Empty,
        }
    }
}

/// 模板节点声明的具体内容来源（严格版；解析失败 = 该节点无内容）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContentSpec {
    Text {
        text: String,
    },
    Field {
        field: String,
        /// 项目数据标量字段作用域；缺省跟随当前文档类（template.kind），再回退 'project'。
        #[serde(default)]
        doc_kind: Option<String>,
    },
    Table {
        /// 项目绑定体系里的 table_key（如默认体系的 parties/basis_files/…）
        source: String,
        /// 过滤：按该表同名列（如 parties 的角色列）的单元格精确匹配
        #[serde(default)]
        party_kind: Option<String>,
        /// 过滤：按该表同名列（如 category 类别列）的单元格精确匹配
        #[serde(default)]
        category: Option<String>,
    },
    Trace {
        /// matrix | test_items
        view: String,
    },
    Slot {
        /// 执行人员未填时回落的提示文案
        placeholder: String,
        #[serde(default)]
        role: Option<String>,
    },
}

impl ContentSpec {
    /// 宽松解析：非对象 / kind 未知 / 字段不全 → None（该节点按无内容处理）。
    pub fn parse(value: &Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }

    /// 对 `text` 里的 `{field_key}` 占位符逐个替换。取不到值 → 保留原文
    /// （作者占位语义：作者写死某字段名，未填时让读者看到 `{xxx}` 即知漏填）。
    /// 仅以 ASCII 的 `{`/`}` 作切分点，字符串按 `&str` 切片，天然安全（多字节安全）。
    pub fn interpolate(text: &str, get: &dyn Fn(&str) -> Option<String>) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            rest = &rest[open + 1..];
            if let Some(close) = rest.find('}') {
                let key = &rest[..close];
                if !key.is_empty() {
                    match get(key) {
                        Some(v) => out.push_str(&v),
                        None => {
                            out.push('{');
                            out.push_str(key);
                            out.push('}');
                        }
                    }
                    rest = &rest[close + 1..];
                    continue;
                }
            }
            // 无闭合括号 / 空 `{}`：按字面输出 `{` 并继续向后找
            out.push('{');
        }
        out.push_str(rest);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(key: &str) -> Option<String> {
        match key {
            "项目代号" => Some("RT25046001".into()),
            "密级" => Some("秘密★10年".into()),
            _ => None,
        }
    }

    #[test]
    fn interpolate_replaces_known_fields_and_keeps_unknown() {
        let text = "标识：{项目代号}，密级：{密级}，缺项：{未填字段}";
        assert_eq!(
            ContentSpec::interpolate(text, &get),
            "标识：RT25046001，密级：秘密★10年，缺项：{未填字段}"
        );
    }

    #[test]
    fn interpolate_handles_no_braces_and_empty_braces() {
        assert_eq!(ContentSpec::interpolate("纯套话", &get), "纯套话");
        assert_eq!(ContentSpec::interpolate("a{}b", &get), "a{}b");
    }

    #[test]
    fn parse_accepts_known_kinds_and_rejects_unknown() {
        let text = serde_json::json!({"kind": "text", "text": "套话"});
        assert!(matches!(
            ContentSpec::parse(&text),
            Some(ContentSpec::Text { .. })
        ));
        let field = serde_json::json!({"kind": "field", "field": "软件名称"});
        assert!(matches!(
            ContentSpec::parse(&field),
            Some(ContentSpec::Field { .. })
        ));
        // 缺 kind / 未知 kind → None
        assert!(ContentSpec::parse(&serde_json::json!({})).is_none());
        assert!(ContentSpec::parse(&serde_json::json!({"kind": "bogus"})).is_none());
    }

    #[test]
    fn block_kind_from_spec_maps_known_and_falls_back() {
        assert_eq!(BlockKind::from_spec(&serde_json::json!({"kind": "text"})), BlockKind::Text);
        assert_eq!(BlockKind::from_spec(&serde_json::json!({"kind": "slot"})), BlockKind::Slot);
        assert_eq!(BlockKind::from_spec(&serde_json::json!({})), BlockKind::Empty);
        assert_eq!(BlockKind::from_spec(&serde_json::json!({"kind": "x"})), BlockKind::Empty);
    }
}
