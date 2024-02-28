//! A general-purpose value wrapper.

use serde::{Deserialize, Serialize};

/// A general-purpose value wrapper.
///
/// This structure can store serializable values. It can be used as an
/// intermediate type to communicate between script engines (e.g. JavaScript)
/// and other modules such as layout engines (e.g. Jinja).
#[derive(Clone, Debug)]
pub enum Value {
    /// Boolean.
    Bool(bool),
    /// Signed integer.
    I64(i64),
    /// Unsigned integer.
    U64(u64),
    /// Float.
    F64(f64),
    /// String.
    Str(String),
    /// Null.
    Unit,
    /// Array.
    Seq(Vec<Self>),
    /// Object.
    Map(Map<String, Self>),
}

impl Value {
    /// Return the type name as a string slice.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::I64(_) => "i64",
            Self::U64(_) => "u64",
            Self::F64(_) => "f64",
            Self::Str(_) => "str",
            Self::Unit => "unit",
            Self::Seq(_) => "seq",
            Self::Map(_) => "map",
        }
    }

    /// Check if the value is a `bool`.
    pub fn is_bool(&self) -> bool {
        match self {
            Self::Bool(_) => true,
            _ => false,
        }
    }

    /// If the value is a `bool`, returns the associated [`bool`].
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Check if the value is a `i64`.
    pub fn is_i64(&self) -> bool {
        match self {
            Self::I64(_) => true,
            _ => false,
        }
    }

    /// If the value is a `i64`, returns the associated [`i64`].
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// Check if the value is a `u64`.
    pub fn is_u64(&self) -> bool {
        match self {
            Self::U64(_) => true,
            _ => false,
        }
    }

    /// If the value is a `u64`, returns the associated [`u64`].
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::U64(v) => Some(*v),
            _ => None,
        }
    }

    /// Check if the value is a `f64`.
    pub fn is_f64(&self) -> bool {
        match self {
            Self::F64(_) => true,
            _ => false,
        }
    }

    /// If the value is a `f64`, returns the associated [`f64`].
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F64(v) => Some(*v),
            _ => None,
        }
    }

    /// Check if the value is a `str`.
    pub fn is_str(&self) -> bool {
        match self {
            Self::Str(_) => true,
            _ => false,
        }
    }

    /// If the value is a `str`, returns the associated `&str`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(v) => Some(v),
            _ => None,
        }
    }

    /// Check if the value is a `()`.
    pub fn is_unit(&self) -> bool {
        match self {
            Self::Unit => true,
            _ => false,
        }
    }

    /// Check if the value is a `seq`.
    pub fn is_seq(&self) -> bool {
        match self {
            Self::Seq(_) => true,
            _ => false,
        }
    }

    /// If the value is a `seq`, returns the associated [`Vec`].
    pub fn as_seq(&self) -> Option<&Vec<Self>> {
        match self {
            Self::Seq(v) => Some(v),
            _ => None,
        }
    }

    /// If the value is a `seq`, returns the associated mutable [`Vec`].
    pub fn as_seq_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Self::Seq(v) => Some(v),
            _ => None,
        }
    }

    /// Check if the value is a `map`.
    pub fn is_map(&self) -> bool {
        match self {
            Self::Map(_) => true,
            _ => false,
        }
    }

    /// If the value is a `map`, returns the associated [`Map`].
    pub fn as_map(&self) -> Option<&Map<String, Self>> {
        match self {
            Self::Map(v) => Some(v),
            _ => None,
        }
    }

    /// If the value is a `map`, returns the associated mutable [`Map`].
    pub fn as_map_mut(&mut self) -> Option<&mut Map<String, Self>> {
        match self {
            Self::Map(v) => Some(v),
            _ => None,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(u), Self::Bool(v)) if u == v => true,
            (Self::I64(u), Self::I64(v)) if u == v => true,
            (Self::U64(u), Self::U64(v)) if u == v => true,
            (Self::F64(u), Self::F64(v)) if u == v => true,
            (Self::Str(u), Self::Str(v)) if u == v => true,
            (Self::Unit, Self::Unit) => true,
            (Self::Seq(u), Self::Seq(v)) if u == v => true,
            (Self::Map(u), Self::Map(v)) if u == v => true,
            _ => false,
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            Self::Bool(v) => serializer.serialize_bool(*v),
            Self::I64(v) => serializer.serialize_i64(*v),
            Self::U64(v) => serializer.serialize_u64(*v),
            Self::F64(v) => serializer.serialize_f64(*v),
            Self::Str(v) => serializer.serialize_str(v),
            Self::Unit => serializer.serialize_unit(),
            Self::Seq(v) => v.serialize(serializer),
            Self::Map(v) => v.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

impl<'de> serde::de::IntoDeserializer<'de, ValueError> for Value {
    type Deserializer = ValueDeserializer;

    fn into_deserializer(self) -> Self::Deserializer {
        ValueDeserializer(self)
    }
}

/// Type used for [`Value`] maps.
pub type Map<K, V> = std::collections::HashMap<K, V>;

/// Error related to [`Value`].
#[derive(Debug)]
pub struct ValueError(String);

impl std::error::Error for ValueError {}

impl std::fmt::Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl serde::ser::Error for ValueError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self(msg.to_string())
    }
}

impl serde::de::Error for ValueError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self(msg.to_string())
    }
}

/// Convert a [`Value`] to a `T`.
pub fn from_value<'de, T>(value: Value) -> Result<T, ValueError>
where
    T: Deserialize<'de>,
{
    T::deserialize(ValueDeserializer(value))
}

/// Convert a `T` to a [`Value`].
pub fn to_value<T>(value: T) -> Result<Value, ValueError>
where
    T: Serialize,
{
    value.serialize(ValueSerializer)
}

/// Serialize any data structure to [`Value`].
struct ValueSerializer;

impl serde::ser::Serializer for ValueSerializer {
    type Error = ValueError;
    type Ok = Value;
    type SerializeMap = ValueSerializeMap;
    type SerializeSeq = ValueSerializeSeq;
    type SerializeStruct = ValueSerializeStruct;
    type SerializeStructVariant = ValueSerializeStructVariant;
    type SerializeTuple = ValueSerializeTuple;
    type SerializeTupleStruct = ValueSerializeTupleStruct;
    type SerializeTupleVariant = ValueSerializeTupleVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::I64(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U64(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::F64(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Unit)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        let variant = variant.to_string();
        let value = value.serialize(self)?;
        Ok(Value::Map(Map::from([(variant, value)])))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ValueSerializeSeq {
            seq: len.map_or_else(Vec::new, Vec::with_capacity),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len)).map(ValueSerializeTuple)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len)).map(ValueSerializeTupleStruct)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(ValueSerializeTupleVariant {
            key: variant.to_string(),
            seq: self.serialize_seq(Some(len))?,
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(ValueSerializeMap {
            map: len.map_or_else(Map::new, Map::with_capacity),
            key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len)).map(ValueSerializeStruct)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(ValueSerializeStructVariant {
            key: variant.to_string(),
            map: self.serialize_map(Some(len))?,
        })
    }
}

/// Value returned from [`ValueSerializer::serialize_seq`].
///
/// Sequences are represented as `[seq...]`.
struct ValueSerializeSeq {
    seq: Vec<Value>,
}

impl serde::ser::SerializeSeq for ValueSerializeSeq {
    type Error = ValueError;
    type Ok = Value;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let value = value.serialize(ValueSerializer)?;
        self.seq.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Seq(self.seq))
    }
}

/// Value returned from [`ValueSerializer::serialize_tuple`].
///
/// Tuples are represented like sequences.
struct ValueSerializeTuple(ValueSerializeSeq);

impl serde::ser::SerializeTuple for ValueSerializeTuple {
    type Error = ValueError;
    type Ok = Value;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        use serde::ser::SerializeSeq;
        self.0.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use serde::ser::SerializeSeq;
        self.0.end()
    }
}

/// Value returned from [`ValueSerializer::serialize_tuple_struct`].
///
/// Tuple structs are represented like sequences.
struct ValueSerializeTupleStruct(ValueSerializeSeq);

impl serde::ser::SerializeTupleStruct for ValueSerializeTupleStruct {
    type Error = ValueError;
    type Ok = Value;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        use serde::ser::SerializeSeq;
        self.0.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use serde::ser::SerializeSeq;
        self.0.end()
    }
}

/// Value returned from [`ValueSerializer::serialize_tuple_variant`].
///
/// Tuple variants are represented as `{ key: [seq...] }`.
struct ValueSerializeTupleVariant {
    key: String,
    seq: ValueSerializeSeq,
}

impl serde::ser::SerializeTupleVariant for ValueSerializeTupleVariant {
    type Error = ValueError;
    type Ok = Value;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        use serde::ser::SerializeSeq;
        self.seq.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use serde::ser::SerializeSeq;
        Ok(Value::Map(Map::from([(self.key, self.seq.end()?)])))
    }
}

/// Value returned from [`ValueSerializer::serialize_map`].
///
/// Maps are represented as `{ key: value, ... }`.
struct ValueSerializeMap {
    map: Map<String, Value>,
    key: Option<String>,
}

impl serde::ser::SerializeMap for ValueSerializeMap {
    type Error = ValueError;
    type Ok = Value;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key = key.serialize(ValueSerializer)?;
        match key {
            Value::Str(v) => self.key = Some(v),
            v => {
                return Err(ValueError(format!(
                    "invalid type: {}, expected str",
                    v.type_name(),
                )))
            },
        }
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key = self.key.take().expect("value must have a key");
        let value = value.serialize(ValueSerializer)?;
        self.map.insert(key, value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.map))
    }
}

/// Value returned from [`ValueSerializer::serialize_struct`].
///
/// Structs are represented like maps.
struct ValueSerializeStruct(ValueSerializeMap);

impl serde::ser::SerializeStruct for ValueSerializeStruct {
    type Error = ValueError;
    type Ok = Value;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        use serde::ser::SerializeMap;
        self.0.serialize_entry(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use serde::ser::SerializeMap;
        self.0.end()
    }
}

/// Value returned from [`ValueSerializer::serialize_struct_variant`].
///
/// Struct variants are represented as `{ key: { k: v, ... } }`.
struct ValueSerializeStructVariant {
    key: String,
    map: ValueSerializeMap,
}

impl serde::ser::SerializeStructVariant for ValueSerializeStructVariant {
    type Error = ValueError;
    type Ok = Value;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        use serde::ser::SerializeMap;
        self.map.serialize_entry(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use serde::ser::SerializeMap;
        Ok(Value::Map(Map::from([(self.key, self.map.end()?)])))
    }
}

/// Deserialize any data structure from [`Value`].
#[doc(hidden)]
pub struct ValueDeserializer(Value);

/// Deserialize any data structure to [`Value`].
impl<'de> serde::de::Deserializer<'de> for ValueDeserializer {
    type Error = ValueError;

    serde::forward_to_deserialize_any! {
        bool
        i8 i16 i32 i64
        u8 u16 u32 u64
        f32 f64
        char
        str
        string
        bytes
        byte_buf
        unit
        unit_struct
        newtype_struct
        seq
        tuple
        tuple_struct
        map
        struct
        enum
        identifier
        ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        use serde::de::value::{MapDeserializer, SeqDeserializer};
        match self.0 {
            Value::Bool(v) => visitor.visit_bool(v),
            Value::I64(v) => visitor.visit_i64(v),
            Value::U64(v) => visitor.visit_u64(v),
            Value::F64(v) => visitor.visit_f64(v),
            Value::Str(v) => visitor.visit_string(v),
            Value::Unit => visitor.visit_unit(),
            Value::Seq(v) => visitor.visit_seq(SeqDeserializer::new(v.into_iter())),
            Value::Map(v) => visitor.visit_map(MapDeserializer::new(v.into_iter())),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Value::Unit => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }
}

/// A visitor that implements [`serde::de::Visitor`] to deserialize [`Value`].
struct ValueVisitor;

impl<'de> serde::de::Visitor<'de> for ValueVisitor {
    type Value = self::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::I64(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::U64(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::F64(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Str(v.to_string()))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Unit)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(v) = seq.next_element()? {
            values.push(v);
        }
        Ok(Value::Seq(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut pairs = Map::new();
        while let Some((k, v)) = map.next_entry()? {
            pairs.insert(k, v);
        }
        Ok(Value::Map(pairs))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    use super::{from_value, to_value, Value};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Struct {
        bool: bool,
        i32: i32,
        u32: u32,
        f32: f32,
        string: String,
        unit: (),
        vec: Vec<i64>,
        hash_map: HashMap<String, u64>,
        option_some: Option<i64>,
        option_none: Option<i64>,
    }

    #[test]
    fn serialize_struct() {
        let foo = Struct {
            bool: true,
            i32: 1,
            u32: 2,
            f32: 3.0,
            string: "bar".into(),
            unit: (),
            vec: [1_i64, 2_i64, 3_i64].into(),
            hash_map: [("baz".into(), 1_u64)].into(),
            option_some: Some(1),
            option_none: None,
        };

        let expected = Value::Map(
            [
                ("bool".into(), Value::Bool(true)),
                ("i32".into(), Value::I64(1)),
                ("u32".into(), Value::U64(2)),
                ("f32".into(), Value::F64(3.0)),
                ("string".into(), Value::Str("bar".into())),
                ("unit".into(), Value::Unit),
                (
                    "vec".into(),
                    Value::Seq([Value::I64(1), Value::I64(2), Value::I64(3)].into()),
                ),
                (
                    "hash_map".into(),
                    Value::Map([("baz".into(), Value::U64(1))].into()),
                ),
                ("option_some".into(), Value::I64(1)),
                ("option_none".into(), Value::Unit),
            ]
            .into(),
        );

        let result = to_value(foo).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn deserialize_struct() {
        let foo = Value::Map(
            [
                ("bool".into(), Value::Bool(true)),
                ("i32".into(), Value::I64(1)),
                ("u32".into(), Value::U64(2)),
                ("f32".into(), Value::F64(3.0)),
                ("string".into(), Value::Str("bar".into())),
                ("unit".into(), Value::Unit),
                (
                    "vec".into(),
                    Value::Seq([Value::I64(1), Value::I64(2), Value::I64(3)].into()),
                ),
                (
                    "hash_map".into(),
                    Value::Map([("baz".into(), Value::U64(1))].into()),
                ),
                ("option_some".into(), Value::I64(1)),
                ("option_none".into(), Value::Unit),
            ]
            .into(),
        );

        let expected = Struct {
            bool: true,
            i32: 1,
            u32: 2,
            f32: 3.0,
            string: "bar".into(),
            unit: (),
            vec: [1_i64, 2_i64, 3_i64].into(),
            hash_map: [("baz".into(), 1_u64)].into(),
            option_some: Some(1),
            option_none: None,
        };

        let result: Struct = from_value(foo).unwrap();

        assert_eq!(result, expected);
    }
}
