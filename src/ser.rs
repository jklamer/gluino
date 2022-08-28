use std::marker::PhantomData;

use crate::spec::{GluinoSpecType, Spec};

enum SerializationError {}

struct GluinoSerializer<T: GluinoSpecType> {
    spec: Spec,
    _d: PhantomData<T>,
}

impl<T: GluinoSpecType> GluinoSerializer<T> {
    fn new() -> GluinoSerializer<T> {
        GluinoSerializer {
            spec: T::get_spec(),
            _d: PhantomData::<T>,
        }
    }
}

enum GluinoValue {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int128(i128),
    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Uint64(u64),
}

// impl<T: GluinoSpecType> serde::Serializer for GluinoSerializer<T> {
//     type Ok = GluinoValue;

//     type Error = SerializationError;

//     type SerializeSeq;

//     type SerializeTuple;

//     type SerializeTupleStruct;

//     type SerializeTupleVariant;

//     type SerializeMap;

//     type SerializeStruct;

//     type SerializeStructVariant;

//     fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_some<S: ?Sized>(self, value: &S) -> Result<Self::Ok, Self::Error>
//     where
//         S: serde::Serialize,
//     {
//         todo!()
//     }

//     fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_unit_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//     ) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_newtype_struct<S: ?Sized>(
//         self,
//         name: &'static str,
//         value: &S,
//     ) -> Result<Self::Ok, Self::Error>
//     where
//         S: serde::Serialize,
//     {
//         todo!()
//     }

//     fn serialize_newtype_variant<S: ?Sized>(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         value: &S,
//     ) -> Result<Self::Ok, Self::Error>
//     where
//         S: serde::Serialize,
//     {
//         todo!()
//     }

//     fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
//         todo!()
//     }

//     fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
//         todo!()
//     }

//     fn serialize_tuple_struct(
//         self,
//         name: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeTupleStruct, Self::Error> {
//         todo!()
//     }

//     fn serialize_tuple_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeTupleVariant, Self::Error> {
//         todo!()
//     }

//     fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
//         todo!()
//     }

//     fn serialize_struct(
//         self,
//         name: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeStruct, Self::Error> {
//         todo!()
//     }

//     fn serialize_struct_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeStructVariant, Self::Error> {
//         todo!()
//     }
// }
