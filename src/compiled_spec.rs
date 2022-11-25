use crate::{
    fingerprint::SpecFingerprint,
    spec::{
        InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size, Spec,
        StringEncodingFmt,
    },
};
use std::{
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
    pub fn fingerprint<'a>(&'a self) -> &'a SpecFingerprint {
        &self.fingerprint
    }

    pub fn structure<'a>(&'a self) -> &'a CompiledSpecStructure {
        &self.structure
    }

    pub fn named_schema<'a>(&'a self) -> &'a HashMap<String, Arc<CompiledSpec>> {
        &self.named_schema
    }

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
            fingerprint: SpecFingerprint::new(&HashMap::new(), &CompiledSpecStructure::Void),
            named_schema: HashMap::with_capacity(0),
            structure: CompiledSpecStructure::Void,
        }
    }

    pub(crate) fn to_spec(&self) -> Spec {
        Self::make_spec(&self.named_schema, &self.structure)
    }

    //turn the structe of a compiled schema in the provided context into a context free spec
    pub(crate) fn make_spec(
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

    fn push_down_resolved(&mut self, name: &String, resolved_spec: &Arc<CompiledSpec>) {
        if self.named_schema.contains_key(name) {
            self.named_schema.insert(name.clone(), resolved_spec.clone());
        }

        match &mut self.structure {
            CompiledSpecStructure::Map {ref mut key_spec, ref mut value_spec , ..} => {
                key_spec.push_down_resolved(name, resolved_spec);
                value_spec.push_down_resolved(name, resolved_spec);
            },
            CompiledSpecStructure::List {ref mut value_spec, ..} => {
                value_spec.push_down_resolved(name, resolved_spec);
            },
            CompiledSpecStructure::Optional(ref mut spec) => {
                spec.push_down_resolved(name, resolved_spec);
            },
            CompiledSpecStructure::Record { ref mut field_to_spec , ..} => {
                for (_, spec) in field_to_spec.iter_mut() {
                    spec.push_down_resolved(name, resolved_spec);
                }
            },
            CompiledSpecStructure::Tuple(ref mut fields) => {
                for field in fields.iter_mut() {
                    field.push_down_resolved(name, resolved_spec);
                }
            },
            CompiledSpecStructure::Enum { ref mut variant_to_spec, ..} => {
                for (_, spec) in variant_to_spec.iter_mut() {
                    spec.push_down_resolved(name, resolved_spec);
                }
            },
            CompiledSpecStructure::Union(ref mut variants) => {
                for variant in variants.iter_mut() {
                    variant.push_down_resolved(name, resolved_spec);
                }
            },
            CompiledSpecStructure::Name(_) => {
                ()
            }
            CompiledSpecStructure::Void |
            CompiledSpecStructure::Bool |
            CompiledSpecStructure::Uint(_) |
            CompiledSpecStructure::Int(_) |
            CompiledSpecStructure::BinaryFloatingPoint(_) |
            CompiledSpecStructure::DecimalFloatingPoint(_) |
            CompiledSpecStructure::Decimal(_) |
            CompiledSpecStructure::String(_, _) |
            CompiledSpecStructure::Bytes(_) => {()}
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

#[derive(Debug, Eq, PartialEq, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
#[strum_discriminants(name(SpecCompileErrorKind))]
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
        fingerprint: SpecFingerprint::new(&named_schema, &structure),
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
                let mut cs = Arc::new(cs);
                context.insert(name.clone(), cs.clone());
                cs.push_down_resolved(&name, &cs);
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
    use crate::{spec::SpecKind, test_utils::get_all_kinds_spec};
    use strum::IntoEnumIterator;

    #[test]
    fn test_compile_uncompile() {
        fn test_spec_compile_cycle(spec: Spec) {
            let s1: Spec = spec;
            let cs1: CompiledSpec = CompiledSpec::compile(s1.clone()).expect("Unable to compile");
            assert_eq!(s1, cs1.to_spec());
        }
        for spec in get_all_kinds_spec() {
            test_spec_compile_cycle(spec)
        }
    }

    #[test]
    fn test_compile_error_cases_kinds() {
        // Create a spec the compiles with every error
        for kind in SpecCompileErrorKind::iter() {
            match kind {
                SpecCompileErrorKind::DuplicateName => vec![
                    Spec::Record(vec![
                        (
                            "field1".into(),
                            Spec::Name {
                                name: "name1".into(),
                                spec: Spec::Int(3).into(),
                            },
                        ),
                        (
                            "field2".into(),
                            Spec::Name {
                                name: "name1".into(),
                                spec: Spec::BinaryFloatingPoint(
                                    InterchangeBinaryFloatingPointFormat::Double,
                                )
                                .into(),
                            },
                        ),
                    ]),
                    Spec::Record(vec![
                        (
                            "field1".into(),
                            Spec::Record(vec![(
                                "inner field 1".into(),
                                Spec::Name {
                                    name: "name1".into(),
                                    spec: Spec::Int(3).into(),
                                },
                            )]),
                        ),
                        (
                            "field2".into(),
                            Spec::Name {
                                name: "name1".into(),
                                spec: Spec::BinaryFloatingPoint(
                                    InterchangeBinaryFloatingPointFormat::Double,
                                )
                                .into(),
                            },
                        ),
                    ]),
                ],
                SpecCompileErrorKind::UndefinedName => vec![
                    Spec::Ref {
                        name: "any name here".into(),
                    },
                    Spec::Enum(vec![
                        (
                            "variant1".into(),
                            Spec::Ref {
                                name: "a name".into(),
                            },
                        ),
                        (
                            "variant2".into(),
                            Spec::Name {
                                name: "a name".into(),
                                spec: Spec::DecimalFloatingPoint(
                                    InterchangeDecimalFloatingPointFormat::Dec128,
                                )
                                .into(),
                            },
                        ),
                    ]),
                ],
                SpecCompileErrorKind::DuplicateRecordFieldNames => vec![Spec::Record(vec![
                    ("field name".into(), Spec::Bool),
                    ("field name".into(), Spec::Int(5)),
                ])],
                SpecCompileErrorKind::DuplicateEnumVariantNames => vec![Spec::Enum(vec![
                    ("variant name".into(), Spec::Bool),
                    ("variant name".into(), Spec::Int(5)),
                ])],
                SpecCompileErrorKind::DuplicateUnionVariantSpecs => vec![
                    Spec::Union(vec![
                        Spec::Name {
                            name: "name".into(),
                            spec: Spec::Bytes(Size::Variable).into(),
                        },
                        Spec::Ref {
                            name: "name".into(),
                        },
                    ]),
                    Spec::Union(vec![Spec::Bool, Spec::Bool]),
                ],
                SpecCompileErrorKind::InfinitelyRecursiveTypes => vec![
                    Spec::Name {
                        name: "outer".into(),
                        spec: Spec::Name {
                            name: "inner".into(),
                            spec: Spec::Enum(vec![
                                (
                                    "variant 1".into(),
                                    Spec::Ref {
                                        name: "outer".into(),
                                    },
                                ),
                                (
                                    "variant 2".into(),
                                    Spec::Ref {
                                        name: "inner".into(),
                                    },
                                ),
                            ])
                            .into(),
                        }
                        .into(),
                    },
                    Spec::Name {
                        name: "name".into(),
                        spec: Spec::Record(vec![
                            ("field 1".into(), Spec::Bool),
                            (
                                "field 2".into(),
                                Spec::Ref {
                                    name: "name".into(),
                                },
                            ),
                        ])
                        .into(),
                    },
                ],
                SpecCompileErrorKind::IllegalDecimalFmt => vec![Spec::Decimal {
                    precision: 3,
                    scale: 4,
                }],
            }
            .into_iter()
            .for_each(|s| {
                let error = s.compile().map_err(|e| SpecCompileErrorKind::from(e));
                match error {
                    Ok(compiled_spec) => {
                        panic!(
                            "Illegal spec compiled successfully into {:?}",
                            compiled_spec
                        )
                    }
                    Err(compiled_error_kind) => {
                        assert_eq!(kind, compiled_error_kind)
                    }
                }
            });
        }
    }

    #[test]
    fn test_recursion() {
        let cs = CompiledSpec::compile(Spec::Name { 
            name: "test".into(), 
            spec: Box::new(Spec::Tuple(
                vec![Spec::Int(3),
                 Spec::Optional(Box::new(Spec::Ref{name:"test".into()}))
                 ]
                )
            )
            }
        ).unwrap();
        if let CompiledSpecStructure::Name(name) = cs.structure() {
            if let CompiledSpecStructure::Tuple(compiled_specs) = cs.named_schema().get("test").unwrap().structure() {
                dbg!(compiled_specs[1].named_schema().get("test"));
            };
        };
    }
}
