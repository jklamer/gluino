use crate::{
    fingerprint::SpecFingerprint,
    spec::{
        InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size, Spec,
        StringEncodingFmt,
    },
};
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::Arc,
};
use strum::{EnumDiscriminants, EnumIter};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CompiledSpec {
    fingerprint: SpecFingerprint,
    named_schema: HashMap<String, Arc<CompiledSpec>>,
    structure: CompiledSpecStructure,
}

impl CompiledSpec {
    pub fn compile(spec: Spec) -> Result<CompiledSpec, SpecCompileError> {
        Self::compile_in_context(spec, &mut HashMap::new())
    }

    pub fn compile_in_context(
        spec: Spec,
        context: &mut HashMap<String, Arc<CompiledSpec>>,
    ) -> Result<CompiledSpec, SpecCompileError> {
        compile_spec_internal(spec, context, &mut HashSet::new(), &mut HashSet::new())
    }

    //internal placeholder compiled spec used for name resolution workflows
    fn invalid_compiled_spec() -> CompiledSpec {
        CompiledSpec {
            fingerprint: SpecFingerprint::new(&Spec::Void),
            named_schema: HashMap::with_capacity(0),
            structure: CompiledSpecStructure::Void,
        }
    }

    pub(crate) fn to_spec(&self) -> Spec {
        Self::make_spec(&self.named_schema, &self.structure)
    }

    fn make_spec(
        context: &HashMap<String, Arc<CompiledSpec>>,
        structure: &CompiledSpecStructure,
    ) -> Spec {
        Self::make_spec_internal(context, &mut HashSet::new(), structure)
    }

    fn make_spec_internal(
        context: &HashMap<String, Arc<CompiledSpec>>,
        names_converted: &mut HashSet<String>,
        structure: &CompiledSpecStructure,
    ) -> Spec {
        match structure {
            CompiledSpecStructure::Void => Spec::Void,
            CompiledSpecStructure::Bool => Spec::Bool,
            CompiledSpecStructure::Uint(n) => Spec::Uint(*n),
            CompiledSpecStructure::Int(n) => Spec::Int(*n),
            CompiledSpecStructure::BinaryFloatingPoint(fmt) => {
                Spec::BinaryFloatingPoint(fmt.clone())
            }
            CompiledSpecStructure::DecimalFloatingPoint(fmt) => {
                Spec::DecimalFloatingPoint(fmt.clone())
            }
            CompiledSpecStructure::Decimal(DecimalFmt { precision, scale }) => Spec::Decimal {
                precision: *precision,
                scale: *scale,
            },
            CompiledSpecStructure::Map {
                size,
                key_spec,
                value_spec,
            } => Spec::Map {
                size: size.clone(),
                key_spec: Box::new(Self::make_spec_internal(
                    context,
                    names_converted,
                    &key_spec.structure,
                )),
                value_spec: Box::new(Self::make_spec_internal(
                    context,
                    names_converted,
                    &value_spec.structure,
                )),
            },
            CompiledSpecStructure::List { size, value_spec } => Spec::List {
                size: size.clone(),
                value_spec: Box::new(Self::make_spec_internal(
                    context,
                    names_converted,
                    &value_spec.structure,
                )),
            },
            CompiledSpecStructure::String(size, fmt) => Spec::String(size.clone(), fmt.clone()),
            CompiledSpecStructure::Bytes(size) => Spec::Bytes(size.clone()),
            CompiledSpecStructure::Optional(s) => Spec::Optional(Box::new(
                Self::make_spec_internal(context, names_converted, &s.structure),
            )),
            CompiledSpecStructure::Name(name) => {
                if names_converted.contains(name) {
                    Spec::Ref { name: name.clone() }
                } else {
                    names_converted.insert(name.clone());
                    Spec::Name {
                        name: name.clone(),
                        spec: Box::new(Self::make_spec_internal(
                            context,
                            names_converted,
                            &context.get(name).unwrap().structure,
                        )),
                    }
                }
            }
            CompiledSpecStructure::Record {
                fields,
                field_to_spec,
                ..
            } => Spec::Record(
                fields
                    .iter()
                    .map(|f| {
                        (
                            f.clone(),
                            Self::make_spec_internal(
                                context,
                                names_converted,
                                &field_to_spec.get(f).unwrap().structure,
                            ),
                        )
                    })
                    .collect(),
            ),
            CompiledSpecStructure::Tuple(compiled_specs) => Spec::Tuple(
                compiled_specs
                    .iter()
                    .map(|cs| Self::make_spec_internal(context, names_converted, &cs.structure))
                    .collect(),
            ),
            CompiledSpecStructure::Enum {
                variants,
                variant_to_spec,
            } => Spec::Enum(
                variants
                    .iter()
                    .map(|f| {
                        (
                            f.clone(),
                            Self::make_spec_internal(
                                context,
                                names_converted,
                                &variant_to_spec.get(f).unwrap().structure,
                            ),
                        )
                    })
                    .collect(),
            ),
            CompiledSpecStructure::Union(compiled_specs) => Spec::Union(
                compiled_specs
                    .iter()
                    .map(|cs| Self::make_spec_internal(context, names_converted, &cs.structure))
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
pub enum CompiledSpecStructure {
    Void,
    Bool,
    Uint(u8),
    Int(u8),
    BinaryFloatingPoint(InterchangeBinaryFloatingPointFormat),
    DecimalFloatingPoint(InterchangeDecimalFloatingPointFormat),
    Decimal(DecimalFmt),
    Map {
        size: Size,
        key_spec: Box<CompiledSpec>,
        value_spec: Box<CompiledSpec>,
    },
    List {
        size: Size,
        value_spec: Box<CompiledSpec>,
    },
    String(Size, StringEncodingFmt),
    Bytes(Size),
    Optional(Box<CompiledSpec>),
    Name(String),
    Record {
        fields: Vec<String>,
        field_to_spec: HashMap<String, CompiledSpec>,
        field_to_index: HashMap<String, usize>,
    },
    Tuple(Vec<CompiledSpec>),
    Enum {
        variants: Vec<String>,
        variant_to_spec: HashMap<String, CompiledSpec>,
    },
    Union(Vec<CompiledSpec>),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum SpecCompileError {
    DuplicateName(String),
    UndefinedName(String),
    DuplicateRecordFieldNames(HashSet<String>),
    DuplicateEnumVariantNames(HashSet<String>),
    DuplicateUnionVariantSpecs(Vec<CompiledSpec>),
    InfinitelyRecursiveTypes(HashSet<String>),
    IllegalDecimalFmt,
}

impl From<IllegalDecimalFmt> for SpecCompileError {
    fn from(_: IllegalDecimalFmt) -> Self {
        SpecCompileError::IllegalDecimalFmt
    }
}

pub(crate) fn compile_spec_internal(
    spec: Spec,
    context: &mut HashMap<String, Arc<CompiledSpec>>,
    non_optional_names: &mut HashSet<String>,
    names_used: &mut HashSet<String>,
) -> Result<CompiledSpec, SpecCompileError> {
    let mut internal_names_used = HashSet::new();
    let structure =
        compile_structure_internal(spec, context, non_optional_names, &mut internal_names_used)?;
    let mut named_schema = HashMap::new();
    for name in internal_names_used.iter() {
        named_schema.insert(name.clone(), context.get(name).unwrap().clone());
    }
    names_used.extend(internal_names_used.into_iter());
    Ok(CompiledSpec {
        fingerprint: SpecFingerprint::new(&CompiledSpec::make_spec(&named_schema, &structure)),
        named_schema,
        structure,
    })
}

pub(crate) fn compile_structure_internal(
    spec: Spec,
    context: &mut HashMap<String, Arc<CompiledSpec>>,
    non_optional_names: &mut HashSet<String>,
    names_used: &mut HashSet<String>,
) -> Result<CompiledSpecStructure, SpecCompileError> {
    match spec {
        Spec::Bool => Ok(CompiledSpecStructure::Bool),
        Spec::Uint(n) => Ok(CompiledSpecStructure::Uint(n)),
        Spec::Int(n) => Ok(CompiledSpecStructure::Int(n)),
        Spec::BinaryFloatingPoint(fmt) => Ok(CompiledSpecStructure::BinaryFloatingPoint(fmt)),
        Spec::DecimalFloatingPoint(fmt) => Ok(CompiledSpecStructure::DecimalFloatingPoint(fmt)),
        Spec::Decimal { precision, scale } => Ok(CompiledSpecStructure::Decimal(DecimalFmt::new(
            precision, scale,
        )?)),
        Spec::Map {
            size,
            key_spec,
            value_spec,
        } => Ok(CompiledSpecStructure::Map {
            size,
            key_spec: box_compile(key_spec, context, names_used)?,
            value_spec: box_compile(value_spec, context, names_used)?,
        }),
        Spec::List { size, value_spec } => Ok(CompiledSpecStructure::List {
            size,
            value_spec: box_compile(value_spec, context, names_used)?,
        }),
        Spec::String(size, fmt) => Ok(CompiledSpecStructure::String(size, fmt)),
        Spec::Bytes(size) => Ok(CompiledSpecStructure::Bytes(size)),
        Spec::Optional(s) => Ok(CompiledSpecStructure::Optional(box_compile(
            s, context, names_used,
        )?)),
        Spec::Name { name, spec } => {
            if context.contains_key(&name) {
                Err(SpecCompileError::DuplicateName(name.clone()))
            } else {
                context.insert(
                    name.clone(),
                    Arc::new(CompiledSpec::invalid_compiled_spec()),
                );
                non_optional_names.insert(name.clone());
                let cs = compile_spec_internal(*spec, context, non_optional_names, names_used)?;
                context.insert(name.clone(), Arc::new(cs));
                non_optional_names.remove(&name);
                names_used.insert(name.clone());
                Ok(CompiledSpecStructure::Name(name))
            }
        }
        Spec::Ref { name } => {
            if non_optional_names.contains(&name) {
                Err(SpecCompileError::InfinitelyRecursiveTypes(HashSet::from([
                    name,
                ])))
            } else if context.contains_key(&name) {
                names_used.insert(name.clone());
                Ok(CompiledSpecStructure::Name(name))
            } else {
                Err(SpecCompileError::UndefinedName(name))
            }
        }
        Spec::Record(fields) => {
            let field_names = fields
                .iter()
                .map(|f| &f.0)
                .map(|name| name.clone())
                .collect();
            let mut duplicate_name_track = HashSet::new();
            let field_to_index = fields
                .iter()
                .enumerate()
                .map(|(index, (name, _))| (name.clone(), index))
                .collect();
            let mut field_to_spec = HashMap::with_capacity(fields.capacity());
            for (field_name, field_spec) in fields {
                if field_to_spec
                    .insert(
                        field_name.clone(),
                        compile_spec_internal(field_spec, context, non_optional_names, names_used)?,
                    )
                    .is_some()
                {
                    duplicate_name_track.insert(field_name);
                }
            }
            if duplicate_name_track.is_empty() {
                Ok(CompiledSpecStructure::Record {
                    fields: field_names,
                    field_to_spec,
                    field_to_index,
                })
            } else {
                Err(SpecCompileError::DuplicateRecordFieldNames(
                    duplicate_name_track,
                ))
            }
        }
        Spec::Tuple(fields) => {
            let mut compiled_fields = Vec::with_capacity(fields.capacity());
            for field_spec in fields {
                compiled_fields.push(compile_spec_internal(
                    field_spec,
                    context,
                    &mut non_optional_names.clone(),
                    names_used,
                )?)
            }
            Ok(CompiledSpecStructure::Tuple(compiled_fields))
        }
        Spec::Enum(variants) => {
            let variant_names: Vec<String> =
                variants.iter().map(|v| &v.0).map(|n| n.clone()).collect();
            let mut all_names: HashSet<String> = variant_names.clone().into_iter().collect();
            let duplicate_names: HashSet<String> = variant_names
                .iter()
                .filter(|&n| !all_names.remove(n))
                .map(|n| n.clone())
                .collect();
            if !duplicate_names.is_empty() {
                return Err(SpecCompileError::DuplicateEnumVariantNames(duplicate_names));
            }
            let mut variant_to_spec = HashMap::new();
            compile_variants_with_loop_checking(
                variants,
                &mut variant_to_spec,
                non_optional_names,
                context,
                names_used,
            )?;
            Ok(CompiledSpecStructure::Enum {
                variants: variant_names,
                variant_to_spec,
            })
        }
        Spec::Union(variants) => {
            let len = variants.len();
            let variants: Vec<(usize, Spec)> = variants.into_iter().enumerate().collect();
            let mut variants_to_spec = HashMap::new();
            compile_variants_with_loop_checking(
                variants,
                &mut variants_to_spec,
                non_optional_names,
                context,
                names_used,
            )?;
            let compiled_variants: Vec<CompiledSpec> = (0..len)
                .map(|index| variants_to_spec.remove(&index).unwrap())
                .collect();
            let mut variant_fingerprints: HashSet<&SpecFingerprint> = HashSet::new();
            let duplicate_variants: Vec<CompiledSpec> = compiled_variants
                .iter()
                .filter(|&v| !variant_fingerprints.insert(&v.fingerprint))
                .map(|v| v.clone())
                .collect();
            if duplicate_variants.is_empty() {
                Ok(CompiledSpecStructure::Union(compiled_variants))
            } else {
                Err(SpecCompileError::DuplicateUnionVariantSpecs(
                    duplicate_variants,
                ))
            }
        }
        Spec::Void => Ok(CompiledSpecStructure::Void),
    }
}

impl TryFrom<Spec> for CompiledSpec {
    type Error = SpecCompileError;
    fn try_from(spec: Spec) -> Result<CompiledSpec, SpecCompileError> {
        CompiledSpec::compile(spec)
    }
}

#[inline]
fn box_compile(
    spec: Box<Spec>,
    context: &mut HashMap<String, Arc<CompiledSpec>>,
    names_used: &mut HashSet<String>,
) -> Result<Box<CompiledSpec>, SpecCompileError> {
    Ok(Box::new(compile_spec_internal(
        *spec,
        context,
        &mut HashSet::new(),
        names_used,
    )?))
}

#[inline]
fn compile_variants_with_loop_checking<T>(
    variants: Vec<(T, Spec)>,
    variant_to_spec: &mut HashMap<T, CompiledSpec>,
    non_optional_names: &mut HashSet<String>,
    context: &mut HashMap<String, Arc<CompiledSpec>>,
    names_used: &mut HashSet<String>,
) -> Result<(), SpecCompileError>
where
    T: Eq + PartialEq + Hash + Clone,
{
    let num_variants = variants.len();
    let mut variants_with_non_optional_name_errors = HashSet::new();
    let mut offending_names_for_all_variants = HashSet::new();
    for (variant_name, variant_spec) in variants {
        let mut non_offending_names_for_variant = non_optional_names.clone();
        let mut offending_names_for_variant = HashSet::new();
        let cs = loop {
            //TODO get rid of clone somehow
            match compile_spec_internal(
                variant_spec.clone(),
                context,
                &mut non_offending_names_for_variant,
                names_used,
            ) {
                Ok(cs) => break Ok(cs),
                Err(SpecCompileError::InfinitelyRecursiveTypes(offending_names)) => {
                    offending_names.iter().for_each(|offending_name| {
                        non_offending_names_for_variant.remove(offending_name);
                    });
                    offending_names.into_iter().for_each(|offending_name| {
                        offending_names_for_variant.insert(offending_name);
                    });
                    variants_with_non_optional_name_errors.insert(variant_name.clone());
                }
                Err(e) => break Err(e),
            }
        }?;
        offending_names_for_variant
            .into_iter()
            .for_each(|offending_name| {
                offending_names_for_all_variants.insert(offending_name);
            });
        variant_to_spec.insert(variant_name.clone(), cs);
    }
    if variants_with_non_optional_name_errors.len() == num_variants {
        Err(SpecCompileError::InfinitelyRecursiveTypes(
            offending_names_for_all_variants,
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct DecimalFmt {
    pub precision: u64,
    pub scale: u64,
}

pub struct IllegalDecimalFmt;

impl DecimalFmt {
    pub fn new(precision: u64, scale: u64) -> Result<DecimalFmt, IllegalDecimalFmt> {
        if scale <= precision {
            Ok(DecimalFmt { precision, scale })
        } else {
            Err(IllegalDecimalFmt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::SpecKind;
    use strum::IntoEnumIterator;

    #[test]
    fn test_compile_uncompile() {
        macro_rules! test_spec_compile_cycle {
            ($spec:expr) => {
                let s1: Spec = $spec;
                let cs1: CompiledSpec =
                    CompiledSpec::compile(s1.clone()).expect("Unable to compile");
                assert_eq!(s1, cs1.to_spec());
            };
        }
        for kind in SpecKind::iter() {
            match kind {
                SpecKind::Bool => {
                    test_spec_compile_cycle!(Spec::Bool);
                }
                SpecKind::Uint => {
                    for i in 0..=u8::MAX {
                        test_spec_compile_cycle!(Spec::Uint(i));
                    }
                }
                SpecKind::Int => {
                    for i in 0..=u8::MAX {
                        test_spec_compile_cycle!(Spec::Int(i));
                    }
                }
                SpecKind::BinaryFloatingPoint => {
                    for bfp in InterchangeBinaryFloatingPointFormat::iter() {
                        test_spec_compile_cycle!(Spec::BinaryFloatingPoint(bfp));
                    }
                }
                SpecKind::DecimalFloatingPoint => {
                    for dfp in InterchangeDecimalFloatingPointFormat::iter() {
                        test_spec_compile_cycle!(Spec::DecimalFloatingPoint(dfp));
                    }
                }
                SpecKind::Decimal => {
                    test_spec_compile_cycle!(Spec::Decimal {
                        precision: 22,
                        scale: 2
                    });
                    test_spec_compile_cycle!(Spec::Decimal {
                        precision: 10,
                        scale: 2
                    });
                    test_spec_compile_cycle!(Spec::Decimal {
                        precision: 77,
                        scale: 10
                    });
                    test_spec_compile_cycle!(Spec::Decimal {
                        precision: 40,
                        scale: 10
                    });
                }
                SpecKind::Map => {
                    test_spec_compile_cycle!(Spec::Map {
                        size: Size::Variable,
                        key_spec: Spec::Bool.into(),
                        value_spec: Spec::Int(4).into()
                    });
                    test_spec_compile_cycle!(Spec::Map {
                        size: Size::Fixed(50),
                        key_spec: Spec::Int(4).into(),
                        value_spec: Spec::Int(4).into()
                    });
                }
                SpecKind::List => {
                    test_spec_compile_cycle!(Spec::List {
                        size: Size::Variable,
                        value_spec: Spec::BinaryFloatingPoint(
                            InterchangeBinaryFloatingPointFormat::Double
                        )
                        .into()
                    });
                    test_spec_compile_cycle!(Spec::List {
                        size: Size::Fixed(32),
                        value_spec: Spec::Decimal {
                            precision: 10,
                            scale: 2
                        }
                        .into()
                    });
                }
                SpecKind::String => {
                    test_spec_compile_cycle!(Spec::String(Size::Variable, StringEncodingFmt::Utf8));
                    for fmt in StringEncodingFmt::iter() {
                        test_spec_compile_cycle!(Spec::String(Size::Fixed(45), fmt.clone()));
                        test_spec_compile_cycle!(Spec::String(Size::Variable, fmt));
                    }
                }
                SpecKind::Bytes => {
                    test_spec_compile_cycle!(Spec::Bytes(Size::Variable));
                    test_spec_compile_cycle!(Spec::Bytes(Size::Fixed(1024)));
                }
                SpecKind::Optional => {
                    test_spec_compile_cycle!(Spec::Optional(Spec::Bytes(Size::Variable).into()));
                    test_spec_compile_cycle!(Spec::Optional(Spec::Int(6).into()));
                }
                SpecKind::Name => {
                    test_spec_compile_cycle!(Spec::Name {
                        name: "test".into(),
                        spec: Spec::List {
                            size: Size::Fixed(32),
                            value_spec: Spec::Decimal {
                                precision: 10,
                                scale: 2
                            }
                            .into()
                        }
                        .into()
                    });
                    test_spec_compile_cycle!(Spec::Name {
                        name: "test".into(),
                        spec: Spec::Bytes(Size::Variable).into(),
                    });
                }
                SpecKind::Ref => {
                    test_spec_compile_cycle!(Spec::Name {
                        name: "test".into(),
                        spec: Box::new(Spec::Record(vec![
                            ("field1".into(), Spec::Bool),
                            ("field2".into(), Spec::Int(4)),
                            (
                                "field3".into(),
                                Spec::Optional(Box::new(Spec::Ref {
                                    name: "test".into()
                                }))
                            )
                        ]))
                    });
                }
                SpecKind::Record => {
                    test_spec_compile_cycle!(Spec::Record(vec![
                        ("field1".into(), Spec::Bool),
                        ("field2".into(), Spec::Int(4))
                    ]));
                }
                SpecKind::Tuple => {
                    test_spec_compile_cycle!(Spec::Tuple(vec![Spec::Bool, Spec::Int(4)]));
                }
                SpecKind::Enum => {
                    test_spec_compile_cycle!(Spec::Enum(vec![
                        ("field1".into(), Spec::Bool),
                        ("field2".into(), Spec::Int(4))
                    ]));
                }
                SpecKind::Union => {
                    test_spec_compile_cycle!(Spec::Union(vec![Spec::Bool, Spec::Int(4)]));
                }
                SpecKind::Void => {
                    test_spec_compile_cycle!(Spec::Void);
                }
            }
        }
    }
}
