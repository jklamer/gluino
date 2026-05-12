use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::spec::Spec;
use crate::spec::SpecType;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct SpecFingerprint {
    bytes: [u8; 32],
}

pub(crate) const PLACE_HOLDER: SpecFingerprint = SpecFingerprint {
    bytes: [0; 32],
};

impl SpecFingerprint {
    pub fn new(
        named_schema: &HashMap<String, Spec>,
        structure: &SpecType,
    ) -> SpecFingerprint {
        let mut hasher = Sha256::new();
        hasher.update(Spec::make_parsed_spec(named_schema, structure).to_bytes());
        let result = hasher.finalize();
        SpecFingerprint {
            bytes: result.into(),
        }
    }
}

impl Debug for SpecFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::with_capacity(32 + 32 / 4);
        s.push_str(&hex::encode(&self.bytes[0..8]));
        for chunk in self.bytes[8..].chunks(8) {
            s.push_str("-");
            s.push_str(&hex::encode(chunk));
        }
        f.write_str(&s)
    }
}

#[cfg(test)]
mod tests {
    use crate::{spec::SpecType, spec_parsing::ParsedSpec};

    use super::*;

    #[test]
    fn smoke() {
        println!(
            "{:?}",
            SpecFingerprint::new(&HashMap::new(), &SpecType::Int(4))
        );
    }

    #[test]
    fn test_name_schema_fingerprint_consistency() {
        let named_spec = ParsedSpec::Name {
            name: "testName".into(),
            spec: ParsedSpec::Bool.into(),
        };
        let named_nested = ParsedSpec::Record(vec![
            ("field 1".into(), named_spec.clone()),
            (
                "field 2".into(),
                ParsedSpec::Ref {
                    name: "testName".into(),
                },
            ),
        ]);
        let named_spec_compiled = Spec::compile(named_spec).unwrap();
        let named_nested_compiled = Spec::compile(named_nested).unwrap();
        if let SpecType::Record { field_to_spec, .. } =
            named_nested_compiled.spec_type()
        {
            if let Some(field_spec) = field_to_spec.get("field 1") {
                assert_eq!(named_spec_compiled.fingerprint(), field_spec.fingerprint())
            } else {
                panic!("wrong")
            }
            if let Some(field_spec) = field_to_spec.get("field 2") {
                assert_eq!(named_spec_compiled.fingerprint(), field_spec.fingerprint())
            } else {
                panic!("wrong")
            }
        } else {
            panic!("wrong")
        }
    }
}
