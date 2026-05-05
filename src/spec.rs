use crate::{
    fingerprint::SpecFingerprint,
    spec_parsing::{
        InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size, ParsedSpec,
        StringEncodingFmt,
    },
};
use core::fmt::Debug;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};
use strum::{EnumDiscriminants, EnumIter};
use crate::serde::GluinoValue;

#[derive(Eq, PartialEq, Clone)]
pub struct Spec {
    pub(crate) fingerprint: SpecFingerprint,
    pub(crate) named_spec: HashMap<String, Spec>,
    pub(crate) spec_type: SpecType,
}

impl Debug for Spec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledSpec")
            .field("fingerprint", &self.fingerprint)
            .field("named_schema", &self.named_spec)
            .field("structure", &self.to_parsed_spec())
            .finish()
    }
}

impl Spec {
    pub fn fingerprint<'a>(&'a self) -> &'a SpecFingerprint {
        &self.fingerprint
    }

    pub fn spec_type<'a>(&'a self) -> &'a SpecType {
        &self.spec_type
    }

    pub fn named_schema<'a>(&'a self) -> &'a HashMap<String, Spec> {
        &self.named_spec
    }

    pub fn compile(spec: ParsedSpec) -> Result<Spec, SpecCompileError> {
        Self::compile_in_context(spec, &mut HashMap::new())
    }

    pub fn compile_in_context(
        parsed_spec: ParsedSpec,
        context: &mut HashMap<String, Spec>,
    ) -> Result<Spec, SpecCompileError> {
        compile_spec_internal(parsed_spec, context, &mut HashSet::new(), &mut HashSet::new())
    }

    //internal placeholder compiled spec used for name resolution workflows
    fn invalid_compiled_spec() -> Spec {
        Spec {
            fingerprint: SpecFingerprint::new(&HashMap::new(), &SpecType::Void),
            named_spec: HashMap::with_capacity(0),
            spec_type: SpecType::Void,
        }
    }

    pub(crate) fn to_parsed_spec(&self) -> ParsedSpec {
        Self::make_parsed_spec(&self.named_spec, &self.spec_type)
    }

    //turn the structe of a compiled schema in the provided context into a context free spec
    pub(crate) fn make_parsed_spec(
        context: &HashMap<String, Spec>,
        structure: &SpecType,
    ) -> ParsedSpec {
        Self::make_parsed_spec_internal(context, &mut HashSet::new(), structure)
    }

    fn make_parsed_spec_internal(
        context: &HashMap<String, Spec>,
        names_converted: &mut HashSet<String>,
        spec_type: &SpecType,
    ) -> ParsedSpec {
        match spec_type {
            SpecType::Void => ParsedSpec::Void,
            SpecType::Bool => ParsedSpec::Bool,
            SpecType::Uint(n) => ParsedSpec::Uint(*n),
            SpecType::Int(n) => ParsedSpec::Int(*n),
            SpecType::BinaryFloatingPoint(fmt) => {
                ParsedSpec::BinaryFloatingPoint(fmt.clone())
            }
            SpecType::DecimalFloatingPoint(fmt) => {
                ParsedSpec::DecimalFloatingPoint(fmt.clone())
            }
            SpecType::Decimal(DecimalFmt { precision, scale }) => ParsedSpec::Decimal {
                precision: *precision,
                scale: *scale,
            },
            SpecType::Map {
                size,
                key_spec,
                value_spec,
            } => ParsedSpec::Map {
                size: size.clone(),
                key_spec: Box::new(Self::make_parsed_spec_internal(
                    context,
                    names_converted,
                    &key_spec.spec_type,
                )),
                value_spec: Box::new(Self::make_parsed_spec_internal(
                    context,
                    names_converted,
                    &value_spec.spec_type,
                )),
            },
            SpecType::List { size, value_spec } => ParsedSpec::List {
                size: size.clone(),
                value_spec: Box::new(Self::make_parsed_spec_internal(
                    context,
                    names_converted,
                    &value_spec.spec_type,
                )),
            },
            SpecType::String(size, fmt) => ParsedSpec::String(size.clone(), fmt.clone()),
            SpecType::Bytes(size) => ParsedSpec::Bytes(size.clone()),
            SpecType::Optional(s) => ParsedSpec::Optional(Box::new(
                Self::make_parsed_spec_internal(context, names_converted, &s.spec_type),
            )),
            SpecType::Name(name) => {
                if names_converted.contains(name) {
                    ParsedSpec::Ref { name: name.clone() }
                } else {
                    names_converted.insert(name.clone());
                    ParsedSpec::Name {
                        name: name.clone(),
                        spec: Box::new(Self::make_parsed_spec_internal(
                            context,
                            names_converted,
                            &context.get(name).unwrap().spec_type,
                        )),
                    }
                }
            }
            SpecType::Record {
                fields,
                field_to_spec,
                ..
            } => ParsedSpec::Record(
                fields
                    .iter()
                    .map(|f| {
                        (
                            f.clone(),
                            Self::make_parsed_spec_internal(
                                context,
                                names_converted,
                                &field_to_spec.get(f).unwrap().spec_type,
                            ),
                        )
                    })
                    .collect(),
            ),
            SpecType::Tuple(compiled_specs) => ParsedSpec::Tuple(
                compiled_specs
                    .iter()
                    .map(|cs| Self::make_parsed_spec_internal(context, names_converted, &cs.spec_type))
                    .collect(),
            ),
            SpecType::Enum {
                variants,
                variant_to_spec,
            } => ParsedSpec::Enum(
                variants
                    .iter()
                    .map(|f| {
                        (
                            f.clone(),
                            Self::make_parsed_spec_internal(
                                context,
                                names_converted,
                                &variant_to_spec.get(f).unwrap().spec_type,
                            ),
                        )
                    })
                    .collect(),
            ),
            SpecType::Union(compiled_specs) => ParsedSpec::Union(
                compiled_specs
                    .iter()
                    .map(|cs| Self::make_parsed_spec_internal(context, names_converted, &cs.spec_type))
                    .collect(),
            ),
        }
    }
}

#[derive(Eq, PartialEq, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
pub enum SpecType {
    Void,
    Bool,
    Uint(u8),
    Int(u8),
    BinaryFloatingPoint(InterchangeBinaryFloatingPointFormat),
    DecimalFloatingPoint(InterchangeDecimalFloatingPointFormat),
    Decimal(DecimalFmt),
    Map {
        size: Size,
        key_spec: Box<Spec>,
        value_spec: Box<Spec>,
    },
    List {
        size: Size,
        value_spec: Box<Spec>,
    },
    String(Size, StringEncodingFmt),
    Bytes(Size),
    Optional(Box<Spec>),
    Name(String),
    Record {
        fields: Vec<String>,
        field_to_spec: HashMap<String, Spec>,
        field_to_index: HashMap<String, usize>,
    },
    Tuple(Vec<Spec>),
    Enum {
        variants: Vec<String>,
        variant_to_spec: HashMap<String, Spec>,
    },
    Union(Vec<Spec>),
    ConstSet(Box<Spec>, Vec<GluinoValue>),
}

#[derive(Debug, Eq, PartialEq, Clone, Hash, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
#[strum_discriminants(name(SpecCompileErrorKind))]
pub enum SpecCompileError {
    DuplicateName(String),
    UndefinedName(String),
    DuplicateRecordFieldNames(HashSet<String>),
    DuplicateEnumVariantNames(HashSet<String>),
    DuplicateUnionVariantSpecs(Vec<Spec>),
    InfinitelyRecursiveTypes(HashSet<String>),
    IllegalDecimalFmt,
    InternalCompilerError(String),
}

impl From<IllegalDecimalFmt> for SpecCompileError {
    fn from(_: IllegalDecimalFmt) -> Self {
        SpecCompileError::IllegalDecimalFmt
    }
}

pub(crate) fn compile_spec_internal(
    spec: ParsedSpec,
    context: &mut HashMap<String, Spec>,
    non_optional_names: &mut HashSet<String>,
    names_used: &mut HashSet<String>,
) -> Result<Spec, SpecCompileError> {
    let mut internal_names_used = HashSet::new();
    let structure =
        compile_structure_internal(spec, context, non_optional_names, &mut internal_names_used)?;
    let mut named_spec = HashMap::new();
    for name in internal_names_used.iter() {
        named_spec.insert(name.clone(), context.get(name).unwrap().clone());
    }
    names_used.extend(internal_names_used.into_iter());
    Ok(Spec {
        fingerprint: SpecFingerprint::new(&named_spec, &structure),
        named_spec,
        spec_type: structure,
    })
}

pub(crate) fn compile_structure_internal(
    spec: ParsedSpec,
    context: &mut HashMap<String, Spec>,
    non_optional_names: &mut HashSet<String>,
    names_used: &mut HashSet<String>,
) -> Result<SpecType, SpecCompileError> {
    match spec {
        ParsedSpec::Bool => Ok(SpecType::Bool),
        ParsedSpec::Uint(n) => Ok(SpecType::Uint(n)),
        ParsedSpec::Int(n) => Ok(SpecType::Int(n)),
        ParsedSpec::BinaryFloatingPoint(fmt) => Ok(SpecType::BinaryFloatingPoint(fmt)),
        ParsedSpec::DecimalFloatingPoint(fmt) => Ok(SpecType::DecimalFloatingPoint(fmt)),
        ParsedSpec::Decimal { precision, scale } => Ok(SpecType::Decimal(DecimalFmt::new(
            precision, scale,
        )?)),
        ParsedSpec::Map {
            size,
            key_spec,
            value_spec,
        } => Ok(SpecType::Map {
            size,
            key_spec: box_compile(key_spec, context, names_used)?,
            value_spec: box_compile(value_spec, context, names_used)?,
        }),
        ParsedSpec::List { size, value_spec } => Ok(SpecType::List {
            size,
            value_spec: box_compile(value_spec, context, names_used)?,
        }),
        ParsedSpec::String(size, fmt) => Ok(SpecType::String(size, fmt)),
        ParsedSpec::Bytes(size) => Ok(SpecType::Bytes(size)),
        ParsedSpec::Optional(s) => Ok(SpecType::Optional(box_compile(
            s, context, names_used,
        )?)),
        ParsedSpec::Name { name, spec } => {
            if context.contains_key(&name) {
                Err(SpecCompileError::DuplicateName(name.clone()))
            } else {
                let compiled_spec_ref = Spec::invalid_compiled_spec();
                context.insert(name.clone(), compiled_spec_ref.clone());
                non_optional_names.insert(name.clone());
                let cs = compile_spec_internal(*spec, context, non_optional_names, names_used)?;
                context.insert(name.clone(), cs);
                non_optional_names.remove(&name);
                names_used.insert(name.clone());
                Ok(SpecType::Name(name))
            }
        }
        ParsedSpec::Ref { name } => {
            if non_optional_names.contains(&name) {
                Err(SpecCompileError::InfinitelyRecursiveTypes(HashSet::from([
                    name,
                ])))
            } else if context.contains_key(&name) {
                names_used.insert(name.clone());
                Ok(SpecType::Name(name))
            } else {
                Err(SpecCompileError::UndefinedName(name))
            }
        }
        ParsedSpec::Record(fields) => {
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
                Ok(SpecType::Record {
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
        ParsedSpec::Tuple(fields) => {
            let mut compiled_fields = Vec::with_capacity(fields.capacity());
            for field_spec in fields {
                compiled_fields.push(compile_spec_internal(
                    field_spec,
                    context,
                    &mut non_optional_names.clone(),
                    names_used,
                )?)
            }
            Ok(SpecType::Tuple(compiled_fields))
        }
        ParsedSpec::Enum(variants) => {
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
            Ok(SpecType::Enum {
                variants: variant_names,
                variant_to_spec,
            })
        }
        ParsedSpec::Union(variants) => {
            let len = variants.len();
            let variants: Vec<(usize, ParsedSpec)> = variants.into_iter().enumerate().collect();
            let mut variants_to_spec = HashMap::new();
            compile_variants_with_loop_checking(
                variants,
                &mut variants_to_spec,
                non_optional_names,
                context,
                names_used,
            )?;
            let compiled_variants: Vec<Spec> = (0..len)
                .map(|index| variants_to_spec.remove(&index).unwrap())
                .collect();
            let mut variant_fingerprints: HashSet<&SpecFingerprint> = HashSet::new();
            let duplicate_variants: Vec<Spec> = compiled_variants
                .iter()
                .filter(|&v| !variant_fingerprints.insert(&v.fingerprint))
                .map(|v| v.clone())
                .collect();
            if duplicate_variants.is_empty() {
                Ok(SpecType::Union(compiled_variants))
            } else {
                Err(SpecCompileError::DuplicateUnionVariantSpecs(
                    duplicate_variants,
                ))
            }
        },
        ParsedSpec::ConstSet(cont_spec, values) => {

        },
        ParsedSpec::Void => Ok(SpecType::Void),
    }
}

impl TryFrom<ParsedSpec> for Spec {
    type Error = SpecCompileError;
    fn try_from(spec: ParsedSpec) -> Result<Spec, SpecCompileError> {
        Spec::compile(spec)
    }
}

#[inline]
fn box_compile(
    spec: Box<ParsedSpec>,
    context: &mut HashMap<String, Spec>,
    names_used: &mut HashSet<String>,
) -> Result<Box<Spec>, SpecCompileError> {
    Ok(Box::new(compile_spec_internal(
        *spec,
        context,
        &mut HashSet::new(),
        names_used,
    )?))
}

#[inline]
fn compile_variants_with_loop_checking<T>(
    variants: Vec<(T, ParsedSpec)>,
    variant_to_spec: &mut HashMap<T, Spec>,
    non_optional_names: &mut HashSet<String>,
    context: &mut HashMap<String, Spec>,
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
    use crate::{spec_parsing::SpecKind, test_utils::get_all_kinds_spec};
    use strum::IntoEnumIterator;

    #[test]
    fn test_compile_uncompile() {
        fn test_spec_compile_cycle(spec: ParsedSpec) {
            let s1: ParsedSpec = spec;
            let cs1: Spec = Spec::compile(s1.clone()).expect("Unable to compile");
            assert_eq!(s1, cs1.to_parsed_spec());
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
                    ParsedSpec::Record(vec![
                        (
                            "field1".into(),
                            ParsedSpec::Name {
                                name: "name1".into(),
                                spec: ParsedSpec::Int(3).into(),
                            },
                        ),
                        (
                            "field2".into(),
                            ParsedSpec::Name {
                                name: "name1".into(),
                                spec: ParsedSpec::BinaryFloatingPoint(
                                    InterchangeBinaryFloatingPointFormat::Double,
                                )
                                .into(),
                            },
                        ),
                    ]),
                    ParsedSpec::Record(vec![
                        (
                            "field1".into(),
                            ParsedSpec::Record(vec![(
                                "inner field 1".into(),
                                ParsedSpec::Name {
                                    name: "name1".into(),
                                    spec: ParsedSpec::Int(3).into(),
                                },
                            )]),
                        ),
                        (
                            "field2".into(),
                            ParsedSpec::Name {
                                name: "name1".into(),
                                spec: ParsedSpec::BinaryFloatingPoint(
                                    InterchangeBinaryFloatingPointFormat::Double,
                                )
                                .into(),
                            },
                        ),
                    ]),
                ],
                SpecCompileErrorKind::UndefinedName => vec![
                    ParsedSpec::Ref {
                        name: "any name here".into(),
                    },
                    ParsedSpec::Enum(vec![
                        (
                            "variant1".into(),
                            ParsedSpec::Ref {
                                name: "a name".into(),
                            },
                        ),
                        (
                            "variant2".into(),
                            ParsedSpec::Name {
                                name: "a name".into(),
                                spec: ParsedSpec::DecimalFloatingPoint(
                                    InterchangeDecimalFloatingPointFormat::Dec128,
                                )
                                .into(),
                            },
                        ),
                    ]),
                ],
                SpecCompileErrorKind::DuplicateRecordFieldNames => vec![ParsedSpec::Record(vec![
                    ("field name".into(), ParsedSpec::Bool),
                    ("field name".into(), ParsedSpec::Int(5)),
                ])],
                SpecCompileErrorKind::DuplicateEnumVariantNames => vec![ParsedSpec::Enum(vec![
                    ("variant name".into(), ParsedSpec::Bool),
                    ("variant name".into(), ParsedSpec::Int(5)),
                ])],
                SpecCompileErrorKind::DuplicateUnionVariantSpecs => vec![
                    ParsedSpec::Union(vec![
                        ParsedSpec::Name {
                            name: "name".into(),
                            spec: ParsedSpec::Bytes(Size::Variable).into(),
                        },
                        ParsedSpec::Ref {
                            name: "name".into(),
                        },
                    ]),
                    ParsedSpec::Union(vec![ParsedSpec::Bool, ParsedSpec::Bool]),
                ],
                SpecCompileErrorKind::InfinitelyRecursiveTypes => vec![
                    ParsedSpec::Name {
                        name: "outer".into(),
                        spec: ParsedSpec::Name {
                            name: "inner".into(),
                            spec: ParsedSpec::Enum(vec![
                                (
                                    "variant 1".into(),
                                    ParsedSpec::Ref {
                                        name: "outer".into(),
                                    },
                                ),
                                (
                                    "variant 2".into(),
                                    ParsedSpec::Ref {
                                        name: "inner".into(),
                                    },
                                ),
                            ])
                            .into(),
                        }
                        .into(),
                    },
                    ParsedSpec::Name {
                        name: "name".into(),
                        spec: ParsedSpec::Record(vec![
                            ("field 1".into(), ParsedSpec::Bool),
                            (
                                "field 2".into(),
                                ParsedSpec::Ref {
                                    name: "name".into(),
                                },
                            ),
                        ])
                        .into(),
                    },
                ],
                SpecCompileErrorKind::IllegalDecimalFmt => vec![ParsedSpec::Decimal {
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
        let cs = Spec::compile(ParsedSpec::Name {
            name: "test".into(),
            spec: Box::new(ParsedSpec::Tuple(vec![
                ParsedSpec::Int(3),
                ParsedSpec::Optional(Box::new(ParsedSpec::Ref {
                    name: "test".into(),
                })),
            ])),
        })
        .unwrap();
        if let SpecType::Name(name) = cs.spec_type() {
            assert!(cs.named_schema().contains_key(name));
            cs.named_schema().get(name).map(|spec| {
                if let SpecType::Tuple(compiled_specs) = spec.spec_type() {
                    assert_ne!(
                        compiled_specs[1].named_schema().get("test").unwrap(),
                        &Spec::invalid_compiled_spec()
                    );
                };
            });
        };
    }
}
