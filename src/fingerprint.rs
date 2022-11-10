use std::collections::HashMap;
use std::sync::Arc;

use sha2::Digest;
use sha2::Sha256;

use crate::compiled_spec::CompiledSpec;
use crate::compiled_spec::CompiledSpecStructure;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct SpecFingerprint {
    bytes: [u8; 32],
}

impl SpecFingerprint {
    pub fn new(
        named_schema: &HashMap<String, Arc<CompiledSpec>>,
        structure: &CompiledSpecStructure,
    ) -> SpecFingerprint {
        let mut hasher = Sha256::new();
        hasher.update(CompiledSpec::make_spec(named_schema, structure).to_longform_bytes());
        let result = hasher.finalize();
        SpecFingerprint {
            bytes: result.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{compiled_spec::CompiledSpecStructure, spec::Spec};

    use super::*;

    #[test]
    fn smoke() {
        println!(
            "{:?}",
            SpecFingerprint::new(&HashMap::new(), &CompiledSpecStructure::Int(4))
        );
    }

    #[test]
    fn test_name_schema_fingerprint_consistency() {
        let named_spec = Spec::Name {
            name: "testName".into(),
            spec: Spec::Bool.into(),
        };
        let named_nested = Spec::Record(vec![
            ("field 1".into(), named_spec.clone()),
            (
                "field 2".into(),
                Spec::Ref {
                    name: "testName".into(),
                },
            ),
        ]);
        let named_spec_compiled = CompiledSpec::compile(named_spec).unwrap();
        let named_nested_compiled = CompiledSpec::compile(named_nested).unwrap();
        if let CompiledSpecStructure::Record { field_to_spec, .. } =
            named_nested_compiled.structure()
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
