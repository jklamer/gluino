use std::iter;

use strum::IntoEnumIterator;

use crate::spec::{
    InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size, Spec,
    SpecKind, StringEncodingFmt,
};

pub(crate) fn get_all_kinds_spec() -> Vec<Spec> {
    let mut specs = Vec::with_capacity(256);
    for spec_kind in SpecKind::iter() {
        for spec in get_valid_specs_for_kind(spec_kind) {
            specs.push(spec);
        }
    }
    assert!(!specs.is_empty());
    specs
}

pub(crate) fn get_valid_specs_for_kind(spec_kind: SpecKind) -> Box<dyn Iterator<Item = Spec>> {
    match spec_kind {
        SpecKind::Bool => Box::new(iter::once(Spec::Bool)),
        SpecKind::Uint => Box::new((0..=u8::MAX).map(|n| Spec::Uint(n))),
        SpecKind::Int => Box::new((0..=u8::MAX).map(|n| Spec::Int(n))),
        SpecKind::BinaryFloatingPoint => Box::new(
            InterchangeBinaryFloatingPointFormat::iter().map(|bfp| Spec::BinaryFloatingPoint(bfp)),
        ),
        SpecKind::DecimalFloatingPoint => Box::new(
            InterchangeDecimalFloatingPointFormat::iter()
                .map(|dfp| Spec::DecimalFloatingPoint(dfp)),
        ),
        SpecKind::Decimal => Box::new(
            vec![
                Spec::Decimal {
                    precision: 22,
                    scale: 2,
                },
                Spec::Decimal {
                    precision: 10,
                    scale: 2,
                },
                Spec::Decimal {
                    precision: 77,
                    scale: 10,
                },
                Spec::Decimal {
                    precision: 40,
                    scale: 10,
                },
            ]
            .into_iter(),
        ),
        SpecKind::Map => Box::new(
            vec![
                Spec::Map {
                    size: Size::Variable,
                    key_spec: Spec::Bool.into(),
                    value_spec: Spec::Int(4).into(),
                },
                Spec::Map {
                    size: Size::Fixed(50),
                    key_spec: Spec::Int(4).into(),
                    value_spec: Spec::Int(4).into(),
                },
            ]
            .into_iter(),
        ),
        SpecKind::List => Box::new(
            vec![
                Spec::List {
                    size: Size::Variable,
                    value_spec: Spec::BinaryFloatingPoint(
                        InterchangeBinaryFloatingPointFormat::Double,
                    )
                    .into(),
                },
                Spec::List {
                    size: Size::Fixed(32),
                    value_spec: Spec::Decimal {
                        precision: 10,
                        scale: 2,
                    }
                    .into(),
                },
            ]
            .into_iter(),
        ),
        SpecKind::String => Box::new(
            iter::once(Spec::String(Size::Variable, StringEncodingFmt::Utf8))
                .chain(StringEncodingFmt::iter().map(|fmt| Spec::String(Size::Fixed(45), fmt)))
                .chain(StringEncodingFmt::iter().map(|fmt| Spec::String(Size::Variable, fmt))),
        ),
        SpecKind::Bytes => Box::new(
            iter::once(Spec::Bytes(Size::Variable))
                .chain(iter::once(Spec::Bytes(Size::Fixed(1024)))),
        ),
        SpecKind::Optional => Box::new(
            iter::once(Spec::Optional(Spec::Bytes(Size::Variable).into()))
                .chain(iter::once(Spec::Optional(Spec::Int(6).into()))),
        ),
        SpecKind::Name => Box::new(
            iter::once(Spec::Name {
                name: "test".into(),
                spec: Spec::List {
                    size: Size::Fixed(32),
                    value_spec: Spec::Decimal {
                        precision: 10,
                        scale: 2,
                    }
                    .into(),
                }
                .into(),
            })
            .chain(iter::once(Spec::Name {
                name: "test".into(),
                spec: Spec::Bytes(Size::Variable).into(),
            })),
        ),
        SpecKind::Ref => Box::new(iter::once(Spec::Name {
            name: "test".into(),
            spec: Box::new(Spec::Record(vec![
                ("field1".into(), Spec::Bool),
                ("field2".into(), Spec::Int(4)),
                (
                    "field3".into(),
                    Spec::Optional(Box::new(Spec::Ref {
                        name: "test".into(),
                    })),
                ),
            ])),
        })),
        SpecKind::Record => Box::new(iter::once(Spec::Record(vec![
            ("field1".into(), Spec::Bool),
            ("field2".into(), Spec::Int(4)),
        ]))),
        SpecKind::Tuple => Box::new(iter::once(Spec::Tuple(vec![Spec::Bool, Spec::Int(4)]))),
        SpecKind::Enum => Box::new(iter::once(Spec::Enum(vec![
            ("field1".into(), Spec::Bool),
            ("field2".into(), Spec::Int(4)),
        ]))),
        SpecKind::Union => Box::new(iter::once(Spec::Union(vec![Spec::Bool, Spec::Int(4)]))),
        SpecKind::Void => Box::new(iter::once(Spec::Void)),
    }
}
