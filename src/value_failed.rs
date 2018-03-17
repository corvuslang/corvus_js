use std::collections::HashMap;
use std::iter::{FromIterator, Map};

use corvus_core::{Block, List as IList, Record as IRecord, Value as IValue, WithError};
use std::error::Error;
use stdweb::{Array, Number, Object, Reference};
use stdweb::web::Date;
use stdweb::unstable::{TryFrom, TryInto};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsValue {
  Boolean(bool),
  Number(Number),
  String(String),
  Time(JsTime),
  List(JsArray),
  Record(JsObject),
  Block(i32),
}

js_deserializable!(JsValue);
js_serializable!(JsValue);


impl From<Block<JsValue>> for JsValue {
  fn from(block: Block<JsValue>) -> JsValue {
    let boxed = Box::new(block);
    let ptr = Box::into_raw(boxed);
    JsValue::Block(ptr as i32)
  }
}

/*
impl From<JsValue> for stdweb::Value {
  fn from(val: JsValue) -> Self {
    match val {
      Boolean(v) => v.into(),
      Number(v) => v.into(),
      String(v) => v.into(),
      Time(v) => v.into(),
      List(v) => v.into(),
      Record(v) => v.into(),
      Block(ptr) => {
        let mut map: HashMap<&'static str, i32> = HashMap::with_capacity(1);
        map.insert("BlockPtr", ptr);
        map.into()
      }
    }
  }
}
*/

/*
impl TryFrom<stdweb::Value> for JsValue {
  fn try_from(v: stdweb::Value) -> Result<Self, String> {
    use stdweb::{InstanceOf, Value};
    let val = match v {
      Value::Null | Value::Undefined => return Err("Corvus has no null or undefined values".into()),
      Value::Symbol(s) => return Err("Corvus does not support symbol values".into()),
      Value::String(s) => JsValue::String(s),
      Value::Bool(s) => JsValue::Boolean(s),
      Value::Number(n) => JsValue::Number(n.into()),
      Value::Reference(r) => {
        if Date::instance_of(&r) {
          JsValue::Time(r)
        } else if Array::instance_of(&r) {
          JsValue::List(r)
        } else {
          JsValue::Record(r)
        }
      }
    };
    Ok(val)
  }
}
*/

impl From<bool> for JsValue {
  fn from(b: bool) -> Self {
    JsValue::Boolean(b)
  }
}

impl From<String> for JsValue {
  fn from(s: String) -> Self {
    JsValue::String(s)
  }
}

/*
impl<T: AsRef<str>> From<T> for JsValue {
  fn from(s: T) -> Self {
    JsValue::String(s.into())
  }
}
*/

impl From<f64> for JsValue {
  fn from(n: f64) -> Self {
    JsValue::Number(Number::from(n))
  }
}

impl From<u64> for JsValue {
  fn from(ms: u64) -> Self {
    JsValue::Time(Date::from_time(ms as f64))
  }
}

impl FromIterator<(String, JsValue)> for JsValue {
  fn from_iter<I: IntoIterator<Item = (String, JsValue)>>(iterable: I) -> Self {
    let map: HashMap<_, Value> = iterable.into_iter().map(|(k, v)| (k, v.into())).collect();
    JsValue::Record(JsObject(Object::from(map).into()))
  }
}

impl FromIterator<JsValue> for JsValue {
  fn from_iter<I: IntoIterator<Item = JsValue>>(iterable: I) -> Self {
    let vec: Vec<stdweb::Value> = iterable.into_iter().map(|v| v.into()).collect();
    JsValue::List(JsArray(Array::from(vec).into()))
  }
}

impl From<Vec<JsValue>> for JsValue {
  fn from(vec: Vec<JsValue>) -> JsValue {
    JsValue::from_iter(vec)
  }
}

impl WithError for JsValue {
  type Error = String;
}

macro_rules! convert_val {
  ($this:ident, $name:ident, $target:ty) => {
    match $this {
      &JsValue::Value(val) => {
        let result: Result<$target, _> = val.try_into();
        result.map_err(|e| e.description().into())
      }
      _ => {
        Err(concat!("tried to convert block into ", stringify!($name)).into())
      }
    }
  };
}

impl IValue for JsValue {
  type List = JsArray;
  type Record = JsObject;

  fn try_bool(&self) -> Result<bool, String> {
    match self {
      &JsValue::Boolean(b) => Ok(b),
      _ => Err("not a bool".into()),
    }
  }

  fn try_string(&self) -> Result<&str, String> {
    match self {
      &JsValue::String(ref s) => Ok(s.as_str()),
      _ => Err("not a string".into()),
    }
  }

  fn try_number(&self) -> Result<f64, String> {
    match self {
      &JsValue::Number(n) => n.try_into()
        .map_err(|e: stdweb::serde::ConversionError| e.description().into()),
      _ => Err("not a number".into()),
    }
  }

  fn try_time(&self) -> Result<u64, String> {
    match self {
      &JsValue::Time(d) => {
        let ms = js!{ return @{d}.getTime(); };
        ms.try_into()
          .map_err(|e: stdweb::serde::ConversionError| e.description().into())
      }
      _ => Err("not a time".into()),
    }
  }

  fn try_list(&self) -> Result<JsArray, String> {
    match self {
      &JsValue::List(array) => Ok(array),
      _ => Err("not an array".into()),
    }
  }

  fn try_record(&self) -> Result<JsObject, String> {
    match self {
      &JsValue::Record(r) => Ok(r),
      _ => Err("not an object".into()),
    }
  }

  fn callable(&self) -> bool {
    if let &JsValue::Block(_) = self {
      true
    } else {
      false
    }
  }

  fn try_call(&self, args: &[Self]) -> Result<Self, String> {
    if let &JsValue::Block(block_ptr) = self {
      call_block_ptr(block_ptr, args)
    } else {
      return Err("not a block".into());
    }
  }
}

pub fn call_block_ptr(ptr: i32, args: &[JsValue]) -> Result<JsValue, String> {
  let block = unsafe { Box::from_raw(ptr as *mut Block<JsValue>) };
  let result = block.call(args);
  Box::into_raw(block); // prevent drop
  result
}


//** Lists

#[derive(Debug, Clone, PartialEq, ReferenceType)]
#[reference(instance_of = "Array")]
pub struct JsArray(Reference);

impl IntoIterator for JsArray {
  type Item = JsValue;
  type IntoIter = Map<JsIter, fn(stdweb::Value) -> JsValue>;

  fn into_iter(self) -> Self::IntoIter {
    console!(log, "into_iter");
    JsIter::new(self.0).map(JsValue::from)
  }
}

impl IList<JsValue> for JsArray {
  fn len(&self) -> usize {
    let res = js!{ return @{self.0}.length; };
    res.try_into().unwrap_or(0)
  }

  fn at(&self, key: usize) -> Option<JsValue> {
    let value = js!{ return @{self.0}[@{key as i32}] };
    nil_to_none(value)
  }
}

//** Records

#[derive(Debug, Clone, PartialEq, ReferenceType, Deserialize, Serialize)]
#[reference(instance_of = "Object")]
pub struct JsObject(Reference);


impl IRecord<JsValue> for JsObject {
  fn at(&self, key: &str) -> Option<JsValue> {
    let value = js!{ return @{self.0}[@{key}] };
    nil_to_none(value)
  }
}

impl IntoIterator for JsObject {
  type Item = (String, JsValue);
  type IntoIter = Map<JsIter, fn(stdweb::Value) -> (String, JsValue)>;

  fn into_iter(self) -> Self::IntoIter {
    JsIter::new(self.0).map(pair_to_tuple)
  }
}

fn pair_to_tuple(pair: stdweb::Value) -> (String, JsValue) {
  let key = js!{ return @{&pair}[0] };
  let val = js!{ return @{&pair}[1] };
  (key.into_string().unwrap(), JsValue::try_from(val).unwrap())
}

fn nil_to_none(value: stdweb::Value) -> Option<JsValue> {
  match value {
    Value::Undefined => None,
    Value::Null => None,
    some => Some(JsValue::from(some)),
  }
}

/// Iterates over a `Reference` using the JS iteration protocol.
pub struct JsIter {
  iter: stdweb::Reference,
}

impl JsIter {
  fn new(value: Reference) -> Self {
    JsIter {
      iter: js!{ return @{value}[Symbol.iterator]() }
        .try_into()
        .unwrap(),
    }
  }
}

js_serializable!(IterNext);
js_deserializable!(IterNext);

impl Iterator for JsIter {
  type Item = stdweb::Value;

  fn next(&mut self) -> Option<stdweb::Value> {
    let result: Result<IterNext, _> = js!{
      return @{&self.iter}.next()
    }.try_into();
    match result {
      Err(_) => None,
      Ok(next) => if next.done {
        None
      } else {
        Some(next.value)
      },
    }
  }
}

#[derive(Serialize, Deserialize)]
struct IterNext {
  value: stdweb::Value,
  done: bool,
}
