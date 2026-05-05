use std::iter;

use strum::IntoEnumIterator;

use crate::spec_parsing::{
    InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size, ParsedSpec,
    SpecKind, StringEncodingFmt,
};

pub(crate) fn get_all_kinds_spec() -> Vec<ParsedSpec> {
    let mut specs = Vec::with_capacity(256);
    for spec_kind in SpecKind::iter() {
        for spec in get_valid_specs_for_kind(spec_kind) {
            specs.push(spec);
        }
    }
    assert!(!specs.is_empty());
    specs
}

pub(crate) fn get_valid_specs_for_kind(spec_kind: SpecKind) -> Box<dyn Iterator<Item =ParsedSpec>> {
    match spec_kind {
        SpecKind::Bool => Box::new(iter::once(ParsedSpec::Bool)),
        SpecKind::Uint => Box::new((0..=u8::MAX).map(|n| ParsedSpec::Uint(n))),
        SpecKind::Int => Box::new((0..=u8::MAX).map(|n| ParsedSpec::Int(n))),
        SpecKind::BinaryFloatingPoint => Box::new(
            InterchangeBinaryFloatingPointFormat::iter().map(|bfp| ParsedSpec::BinaryFloatingPoint(bfp)),
        ),
        SpecKind::DecimalFloatingPoint => Box::new(
            InterchangeDecimalFloatingPointFormat::iter()
                .map(|dfp| ParsedSpec::DecimalFloatingPoint(dfp)),
        ),
        SpecKind::Decimal => Box::new(
            vec![
                ParsedSpec::Decimal {
                    precision: 22,
                    scale: 2,
                },
                ParsedSpec::Decimal {
                    precision: 10,
                    scale: 2,
                },
                ParsedSpec::Decimal {
                    precision: 77,
                    scale: 10,
                },
                ParsedSpec::Decimal {
                    precision: 40,
                    scale: 10,
                },
            ]
            .into_iter(),
        ),
        SpecKind::Map => Box::new(
            vec![
                ParsedSpec::Map {
                    size: Size::Variable,
                    key_spec: ParsedSpec::Bool.into(),
                    value_spec: ParsedSpec::Int(4).into(),
                },
                ParsedSpec::Map {
                    size: Size::Fixed(50),
                    key_spec: ParsedSpec::Int(4).into(),
                    value_spec: ParsedSpec::Int(4).into(),
                },
            ]
            .into_iter(),
        ),
        SpecKind::List => Box::new(
            vec![
                ParsedSpec::List {
                    size: Size::Variable,
                    value_spec: ParsedSpec::BinaryFloatingPoint(
                        InterchangeBinaryFloatingPointFormat::Double,
                    )
                    .into(),
                },
                ParsedSpec::List {
                    size: Size::Fixed(32),
                    value_spec: ParsedSpec::Decimal {
                        precision: 10,
                        scale: 2,
                    }
                    .into(),
                },
            ]
            .into_iter(),
        ),
        SpecKind::String => Box::new(
            iter::once(ParsedSpec::String(Size::Variable, StringEncodingFmt::Utf8))
                .chain(StringEncodingFmt::iter().map(|fmt| ParsedSpec::String(Size::Fixed(45), fmt)))
                .chain(StringEncodingFmt::iter().map(|fmt| ParsedSpec::String(Size::Variable, fmt))),
        ),
        SpecKind::Bytes => Box::new(
            iter::once(ParsedSpec::Bytes(Size::Variable))
                .chain(iter::once(ParsedSpec::Bytes(Size::Fixed(1024)))),
        ),
        SpecKind::Optional => Box::new(
            iter::once(ParsedSpec::Optional(ParsedSpec::Bytes(Size::Variable).into()))
                .chain(iter::once(ParsedSpec::Optional(ParsedSpec::Int(6).into()))),
        ),
        SpecKind::Name => Box::new(
            iter::once(ParsedSpec::Name {
                name: "test".into(),
                spec: ParsedSpec::List {
                    size: Size::Fixed(32),
                    value_spec: ParsedSpec::Decimal {
                        precision: 10,
                        scale: 2,
                    }
                    .into(),
                }
                .into(),
            })
            .chain(iter::once(ParsedSpec::Name {
                name: "test".into(),
                spec: ParsedSpec::Bytes(Size::Variable).into(),
            })),
        ),
        SpecKind::Ref => Box::new(iter::once(ParsedSpec::Name {
            name: "test".into(),
            spec: Box::new(ParsedSpec::Record(vec![
                ("field1".into(), ParsedSpec::Bool),
                ("field2".into(), ParsedSpec::Int(4)),
                (
                    "field3".into(),
                    ParsedSpec::Optional(Box::new(ParsedSpec::Ref {
                        name: "test".into(),
                    })),
                ),
            ])),
        })),
        SpecKind::Record => Box::new(iter::once(ParsedSpec::Record(vec![
            ("field1".into(), ParsedSpec::Bool),
            ("field2".into(), ParsedSpec::Int(4)),
        ]))),
        SpecKind::Tuple => Box::new(iter::once(ParsedSpec::Tuple(vec![ParsedSpec::Bool, ParsedSpec::Int(4)]))),
        SpecKind::Enum => Box::new(iter::once(ParsedSpec::Enum(vec![
            ("field1".into(), ParsedSpec::Bool),
            ("field2".into(), ParsedSpec::Int(4)),
        ]))),
        SpecKind::Union => Box::new(iter::once(ParsedSpec::Union(vec![ParsedSpec::Bool, ParsedSpec::Int(4)]))),
        SpecKind::Void => Box::new(iter::once(ParsedSpec::Void)),
        SpecKind::ConstSet => Box::new(iter::once(ParsedSpec::ConstSet(Box::new(ParsedSpec::Int(2)), Vec::new(Vec::)))),
    }
}
