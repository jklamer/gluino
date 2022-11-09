use sha2::Digest;
use sha2::Sha256;

use crate::compiled_spec::CompiledSpec;
use crate::spec::Spec;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct SpecFingerprint {
    bytes: [u8; 32],
}

impl SpecFingerprint {
    pub fn new(spec: &Spec) -> SpecFingerprint {
        let mut hasher = Sha256::new();
        hasher.update(spec.to_longform_bytes());
        let result = hasher.finalize();
        SpecFingerprint {
            bytes: result.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compiled_spec::CompiledSpec, spec::Spec};

    #[test]
    fn hey() {
        println!("{:?}", SpecFingerprint::new(&Spec::Int(4)));
    }
}
