//! jsonrpc id field

use serde::{Deserialize, Serialize};

/// Request Id
#[derive(Debug, PartialEq, Clone, Hash, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Id {
    /// No id (notification)
    Null,
    /// Numeric id
    Number(u64),
    /// String id
    String(String),
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

    #[test]
    fn id_deserialization() {
        let s = r#""2""#;
        let deserialized: Id = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized, Id::String("2".into()));

        let s = r#"2"#;
        let deserialized: Id = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized, Id::Number(2));

        let s = r#""2x""#;
        let deserialized: Id = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized, Id::String("2x".to_owned()));

        let s = r#"[null, 0, 2, "3"]"#;
        let deserialized: Vec<Id> = serde_json::from_str(s).unwrap();
        assert_eq!(
            deserialized,
            vec![
                Id::Null,
                Id::Number(0),
                Id::Number(2),
                Id::String("3".into())
            ]
        );
    }

    #[test]
    fn id_serialization() {
        let d = vec![
            Id::Null,
            Id::Number(0),
            Id::Number(2),
            Id::Number(3),
            Id::String("3".to_owned()),
            Id::String("test".to_owned()),
        ];
        let serialized = serde_json::to_string(&d).unwrap();
        assert_eq!(serialized, r#"[null,0,2,3,"3","test"]"#);
    }
}
