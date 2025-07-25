use std::str::from_utf8;

use postcard_schema_ng::schema::owned::{OwnedData, OwnedDataModelType};
use serde_json::{Map, Number, Value};

use crate::de::varint::de_zig_zag_i16;

use self::varint::{
    de_zig_zag_i128, de_zig_zag_i32, de_zig_zag_i64, try_take_varint_u128, try_take_varint_u16,
    try_take_varint_u32, try_take_varint_u64, try_take_varint_usize,
};

#[derive(Debug, PartialEq)]
pub enum Error {
    UnexpectedEndOfData,
    ShouldSupportButDont,
    SchemaMismatch,
}

trait GetExt {
    type Out;
    fn right(self) -> Result<Self::Out, Error>;
}

impl<T> GetExt for Option<T> {
    type Out = T;

    fn right(self) -> Result<Self::Out, Error> {
        self.ok_or(Error::SchemaMismatch)
    }
}

pub fn from_slice_dyn(schema: &OwnedDataModelType, data: &[u8]) -> Result<Value, Error> {
    let (val, _remain) = deserialize(schema, data)?;
    Ok(val)
}

fn deserialize<'a>(ty: &OwnedDataModelType, data: &'a [u8]) -> Result<(Value, &'a [u8]), Error> {
    match ty {
        OwnedDataModelType::Bool => {
            let (one, rest) = data.take_one()?;
            let val = match one {
                0 => Value::Bool(false),
                1 => Value::Bool(true),
                _ => return Err(Error::SchemaMismatch),
            };
            Ok((val, rest))
        }
        OwnedDataModelType::I8 => {
            let (one, rest) = data.take_one()?;
            let val = Value::Number(Number::from(one as i8));
            Ok((val, rest))
        }
        OwnedDataModelType::U8 => {
            let (one, rest) = data.take_one()?;
            let val = Value::Number(Number::from(one));
            Ok((val, rest))
        }
        OwnedDataModelType::I16 => {
            let (val, rest) = try_take_varint_u16(data)?;
            let val = de_zig_zag_i16(val);
            let val = Value::Number(Number::from(val));
            Ok((val, rest))
        }
        OwnedDataModelType::I32 => {
            let (val, rest) = try_take_varint_u32(data)?;
            let val = de_zig_zag_i32(val);
            let val = Value::Number(Number::from(val));
            Ok((val, rest))
        }
        OwnedDataModelType::I64 => {
            let (val, rest) = try_take_varint_u64(data)?;
            let val = de_zig_zag_i64(val);
            let val = Value::Number(Number::from(val));
            Ok((val, rest))
        }
        OwnedDataModelType::I128 => {
            let (val, rest) = try_take_varint_u128(data)?;
            let val = de_zig_zag_i128(val);
            let val = i64::try_from(val).map_err(|_| Error::ShouldSupportButDont)?;
            let val = Value::Number(Number::from(val));
            Ok((val, rest))
        }
        OwnedDataModelType::U16 => {
            let (val, rest) = try_take_varint_u16(data)?;
            let val = Value::Number(Number::from(val));
            Ok((val, rest))
        }
        OwnedDataModelType::U32 => {
            let (val, rest) = try_take_varint_u32(data)?;
            let val = Value::Number(Number::from(val));
            Ok((val, rest))
        }
        OwnedDataModelType::U64 => {
            let (val, rest) = try_take_varint_u64(data)?;
            let val = Value::Number(Number::from(val));
            Ok((val, rest))
        }
        OwnedDataModelType::U128 => {
            let (val, rest) = try_take_varint_u128(data)?;
            let val = u64::try_from(val).map_err(|_| Error::ShouldSupportButDont)?;
            let val = Value::Number(Number::from(val));
            Ok((val, rest))
        }
        OwnedDataModelType::Usize => {
            let (val, rest) = try_take_varint_usize(data)?;
            let val = Value::Number(Number::from(val));
            Ok((val, rest))
        }
        OwnedDataModelType::Isize => {
            let (val, rest) = try_take_varint_usize(data)?;

            #[cfg(target_pointer_width = "16")]
            let valu = de_zig_zag_i16(val as u16);

            #[cfg(target_pointer_width = "32")]
            let valu = de_zig_zag_i32(val as u32);

            #[cfg(target_pointer_width = "64")]
            let valu = de_zig_zag_i64(val as u64);

            let valu = Value::Number(Number::from(valu));
            Ok((valu, rest))
        }
        OwnedDataModelType::F32 => {
            let (val, rest) = data.take_n(4)?;
            let mut buf = [0u8; 4];
            buf.copy_from_slice(val);
            let f = f32::from_le_bytes(buf);
            let val = Value::Number(Number::from_f64(f.into()).right()?);
            Ok((val, rest))
        }
        OwnedDataModelType::F64 => {
            let (val, rest) = data.take_n(8)?;
            let mut buf = [0u8; 8];
            buf.copy_from_slice(val);
            let f = f64::from_le_bytes(buf);
            let val = Value::Number(Number::from_f64(f).right()?);
            Ok((val, rest))
        }
        OwnedDataModelType::Char => todo!(),
        OwnedDataModelType::String => {
            let (val, rest) = try_take_varint_usize(data)?;
            let (bytes, rest) = rest.take_n(val)?;
            let s = from_utf8(bytes).map_err(|_| Error::SchemaMismatch)?;
            let val = Value::String(s.to_string());
            Ok((val, rest))
        }
        OwnedDataModelType::ByteArray => {
            let (val, rest) = try_take_varint_usize(data)?;
            let (bytes, rest) = rest.take_n(val)?;
            let vvec = bytes
                .iter()
                .map(|b| Value::Number(Number::from(*b)))
                .collect::<Vec<Value>>();
            let val = Value::Array(vvec);
            Ok((val, rest))
        }
        OwnedDataModelType::Option(inner) => {
            let (val, rest) = data.take_one()?;
            match val {
                0 => return Ok((Value::Null, rest)),
                1 => {}
                _ => return Err(Error::SchemaMismatch),
            }
            deserialize(inner, rest)
        }
        OwnedDataModelType::Unit
        | OwnedDataModelType::Struct {
            name: _,
            data: OwnedData::Unit,
        } => {
            // TODO This is PROBABLY wrong, as Some(()) will be coalesced into the same
            // value as None. Fix this when we have our own Value
            Ok((Value::Null, data))
        }
        OwnedDataModelType::Struct {
            name: _,
            data: OwnedData::Newtype(ty),
        } => deserialize(ty, data),
        OwnedDataModelType::Seq(ty) => {
            let (val, mut rest) = try_take_varint_usize(data)?;
            let mut vec = vec![];
            for _ in 0..val {
                let (v, irest) = deserialize(ty, rest)?;
                rest = irest;
                vec.push(v);
            }
            Ok((Value::Array(vec), rest))
        }
        OwnedDataModelType::Tuple(tys)
        | OwnedDataModelType::Struct {
            name: _,
            data: OwnedData::Tuple(tys),
        } => {
            match &tys[..] {
                [] => {
                    // TODO: Not sure this is right...
                    Ok((Value::Null, data))
                }
                [ty] => {
                    // Single item, NOT an array
                    deserialize(ty, data)
                }
                multi => {
                    let mut vec = vec![];
                    let mut rest = data;
                    for ty in multi.iter() {
                        let (val, irest) = deserialize(ty, rest)?;
                        rest = irest;
                        vec.push(val);
                    }
                    Ok((Value::Array(vec), rest))
                }
            }
        }
        OwnedDataModelType::Map { key, val } => {
            // TODO: impling blind because we can't test this, oops
            //
            // TODO: There's also a mismatch here because serde_json::Value requires
            // keys to be strings, when postcard doesn't.
            if **key != OwnedDataModelType::String {
                return Err(Error::ShouldSupportButDont);
            }

            let (map_len, mut rest) = try_take_varint_usize(data)?;
            let mut map = Map::new();

            for _ in 0..map_len {
                let (str_len, irest) = try_take_varint_usize(rest)?;
                let (bytes, irest) = irest.take_n(str_len)?;
                let s = from_utf8(bytes).map_err(|_| Error::SchemaMismatch)?;

                let (v, irest) = deserialize(val, irest)?;
                rest = irest;

                map.insert(s.to_string(), v);
            }

            Ok((Value::Object(map), rest))
        }
        OwnedDataModelType::Struct {
            name: _,
            data: OwnedData::Struct(nvs),
        } => {
            let mut map = Map::new();
            let mut rest = data;
            for nv in nvs.iter() {
                let (val, irest) = deserialize(&nv.ty, rest)?;
                rest = irest;
                map.insert(nv.name.to_string(), val);
            }
            Ok((Value::Object(map), rest))
        }
        OwnedDataModelType::Enum {
            name: _,
            variants: nvars,
        } => {
            let (variant, rest) = try_take_varint_usize(data)?;
            let schema = nvars.get(variant).right()?;
            match &schema.data {
                OwnedData::Unit => {
                    // Units become strings
                    Ok((Value::String(schema.name.to_string()), rest))
                }
                OwnedData::Newtype(ty) => {
                    // everything else becomes an object with one field
                    let (val, irest) = deserialize(ty, rest)?;
                    let mut map = Map::new();
                    map.insert(schema.name.to_owned().to_string(), val);
                    Ok((Value::Object(map), irest))
                }
                OwnedData::Tuple(vec) => {
                    // everything else becomes an object with one field
                    let (val, irest) = deserialize(&OwnedDataModelType::Tuple(vec.clone()), rest)?;
                    let mut map = Map::new();
                    map.insert(schema.name.to_owned().to_string(), val);
                    Ok((Value::Object(map), irest))
                }
                OwnedData::Struct(vec) => {
                    // everything else becomes an object with one field
                    let (val, irest) = deserialize(
                        &OwnedDataModelType::Struct {
                            name: schema.name.clone(),
                            data: OwnedData::Struct(vec.clone()),
                        },
                        rest,
                    )?;
                    let mut map = Map::new();
                    map.insert(schema.name.to_owned().to_string(), val);
                    Ok((Value::Object(map), irest))
                }
            }
        }
        OwnedDataModelType::Schema => todo!(),
    }
}

mod varint {
    // copy and paste from postcard

    use crate::ser::varint::varint_max;

    use super::{Error, TakeExt};

    /// Returns the maximum value stored in the last encoded byte.
    pub const fn max_of_last_byte<T: Sized>() -> u8 {
        let max_bits = core::mem::size_of::<T>() * 8;
        let extra_bits = max_bits % 7;
        (1 << extra_bits) - 1
    }

    pub fn de_zig_zag_i16(n: u16) -> i16 {
        ((n >> 1) as i16) ^ (-((n & 0b1) as i16))
    }

    pub fn de_zig_zag_i32(n: u32) -> i32 {
        ((n >> 1) as i32) ^ (-((n & 0b1) as i32))
    }

    pub fn de_zig_zag_i64(n: u64) -> i64 {
        ((n >> 1) as i64) ^ (-((n & 0b1) as i64))
    }

    pub fn de_zig_zag_i128(n: u128) -> i128 {
        ((n >> 1) as i128) ^ (-((n & 0b1) as i128))
    }

    #[cfg(target_pointer_width = "16")]
    #[inline(always)]
    pub fn try_take_varint_usize(data: &[u8]) -> Result<(usize, &[u8]), Error> {
        try_take_varint_u16(data).map(|(u, rest)| (u as usize, rest))
    }

    #[cfg(target_pointer_width = "32")]
    #[inline(always)]
    pub fn try_take_varint_usize(data: &[u8]) -> Result<(usize, &[u8]), Error> {
        try_take_varint_u32(data).map(|(u, rest)| (u as usize, rest))
    }

    #[cfg(target_pointer_width = "64")]
    #[inline(always)]
    pub fn try_take_varint_usize(data: &[u8]) -> Result<(usize, &[u8]), Error> {
        try_take_varint_u64(data).map(|(u, rest)| (u as usize, rest))
    }

    #[inline]
    pub fn try_take_varint_u16(data: &[u8]) -> Result<(u16, &[u8]), Error> {
        let mut rest = data;
        let mut out = 0;
        for i in 0..varint_max::<u16>() {
            let (val, later) = rest.take_one()?;
            rest = later;
            let carry = (val & 0x7F) as u16;
            out |= carry << (7 * i);

            if (val & 0x80) == 0 {
                if i == varint_max::<u16>() - 1 && val > max_of_last_byte::<u16>() {
                    return Err(Error::SchemaMismatch);
                } else {
                    return Ok((out, rest));
                }
            }
        }
        Err(Error::SchemaMismatch)
    }

    #[inline]
    pub fn try_take_varint_u32(data: &[u8]) -> Result<(u32, &[u8]), Error> {
        let mut rest = data;
        let mut out = 0;
        for i in 0..varint_max::<u32>() {
            let (val, later) = rest.take_one()?;
            rest = later;
            let carry = (val & 0x7F) as u32;
            out |= carry << (7 * i);

            if (val & 0x80) == 0 {
                if i == varint_max::<u32>() - 1 && val > max_of_last_byte::<u32>() {
                    return Err(Error::SchemaMismatch);
                } else {
                    return Ok((out, rest));
                }
            }
        }
        Err(Error::SchemaMismatch)
    }

    #[inline]
    pub fn try_take_varint_u64(data: &[u8]) -> Result<(u64, &[u8]), Error> {
        let mut rest = data;
        let mut out = 0;
        for i in 0..varint_max::<u64>() {
            let (val, later) = rest.take_one()?;
            rest = later;
            let carry = (val & 0x7F) as u64;
            out |= carry << (7 * i);

            if (val & 0x80) == 0 {
                if i == varint_max::<u64>() - 1 && val > max_of_last_byte::<u64>() {
                    return Err(Error::SchemaMismatch);
                } else {
                    return Ok((out, rest));
                }
            }
        }
        Err(Error::SchemaMismatch)
    }

    #[inline]
    pub fn try_take_varint_u128(data: &[u8]) -> Result<(u128, &[u8]), Error> {
        let mut rest = data;
        let mut out = 0;
        for i in 0..varint_max::<u128>() {
            let (val, later) = rest.take_one()?;
            rest = later;
            let carry = (val & 0x7F) as u128;
            out |= carry << (7 * i);

            if (val & 0x80) == 0 {
                if i == varint_max::<u128>() - 1 && val > max_of_last_byte::<u128>() {
                    return Err(Error::SchemaMismatch);
                } else {
                    return Ok((out, rest));
                }
            }
        }
        Err(Error::SchemaMismatch)
    }
}

trait TakeExt {
    fn take_one(&self) -> Result<(u8, &[u8]), Error>;
    fn take_n(&self, n: usize) -> Result<(&[u8], &[u8]), Error>;
}

impl TakeExt for [u8] {
    fn take_one(&self) -> Result<(u8, &[u8]), Error> {
        if let Some((first, rest)) = self.split_first() {
            Ok((*first, rest))
        } else {
            Err(Error::UnexpectedEndOfData)
        }
    }

    fn take_n(&self, n: usize) -> Result<(&[u8], &[u8]), Error> {
        if self.len() < n {
            return Err(Error::UnexpectedEndOfData);
        }
        Ok(self.split_at(n))
    }
}

#[cfg(test)]
mod test {
    use postcard_schema_ng::Schema;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::{from_slice_dyn, to_stdvec_dyn};

    #[derive(Serialize, Deserialize, Schema)]
    struct UnitStruct;

    #[derive(Serialize, Deserialize, Schema)]
    struct TupStruct1(u8);

    #[derive(Serialize, Deserialize, Schema)]
    struct TupStruct2(u8, u8);

    #[derive(Serialize, Deserialize, Schema)]
    struct Struct1 {
        pub x: bool,
        pub y: u16,
        pub z: f64,
    }

    #[derive(Serialize, Deserialize, Schema)]
    enum Enum1 {
        Alpha,
        Beta(u8),
        Gamma(Vec<u8>),
        Delta(Struct1),
        Epsilon(u8, bool),
        // TODO: struct variants are broken in the Schema derive in
        // stable postcard, tho it is fixed on the main branch.
        // Zeta { a: u8, b: bool },
    }

    #[test]
    fn smoke() {
        let bye = serde_json::to_value(Enum1::Alpha).unwrap();
        let t = to_stdvec_dyn(&Enum1::SCHEMA.into(), &bye).unwrap();
        assert_eq!(vec![0], t);
        let de = from_slice_dyn(&Enum1::SCHEMA.into(), &t).unwrap();
        assert_eq!(
            de,
            json! {
                "Alpha"
            }
        );

        let bye = serde_json::to_value(Enum1::Beta(4)).unwrap();
        let t = to_stdvec_dyn(&Enum1::SCHEMA.into(), &bye).unwrap();
        assert_eq!(vec![1, 4], t);
        let de = from_slice_dyn(&Enum1::SCHEMA.into(), &t).unwrap();
        assert_eq!(
            de,
            json! {
                {"Beta": 4}
            }
        );

        let bye = serde_json::to_value(Enum1::Gamma(vec![1, 2, 3])).unwrap();
        let t = to_stdvec_dyn(&Enum1::SCHEMA.into(), &bye).unwrap();
        assert_eq!(vec![2, 3, 1, 2, 3], t);
        let de = from_slice_dyn(&Enum1::SCHEMA.into(), &t).unwrap();
        assert_eq!(
            de,
            json! {
                {"Gamma": [1, 2, 3]}
            }
        );

        let bye = serde_json::to_value(Enum1::Delta(Struct1 {
            x: false,
            y: 1000,
            z: 4.0,
        }))
        .unwrap();
        let t = to_stdvec_dyn(&Enum1::SCHEMA.into(), &bye).unwrap();
        assert_eq!(vec![3, 0, 232, 7, 0, 0, 0, 0, 0, 0, 16, 64], t);
        let de = from_slice_dyn(&Enum1::SCHEMA.into(), &t).unwrap();
        assert_eq!(
            de,
            json! {
                {"Delta": {
                    "x": false,
                    "y": 1000,
                    "z": 4.0
                }}
            }
        );

        let bye = serde_json::to_value(Enum1::Epsilon(8, false)).unwrap();
        let t = to_stdvec_dyn(&Enum1::SCHEMA.into(), &bye).unwrap();
        assert_eq!(vec![4, 8, 0], t);
        let de = from_slice_dyn(&Enum1::SCHEMA.into(), &t).unwrap();
        assert_eq!(
            de,
            json! {
                {"Epsilon": [8, false]}
            }
        );
    }
}
