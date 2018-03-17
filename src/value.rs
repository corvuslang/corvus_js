use std::collections::HashMap;
use std::iter::{FromIterator, Map};

use corvus_core::{Block, List as IList, Record as IRecord, Value as IValue, WithError};
use stdweb::{Array, Object, Reference, Value};
use stdweb::web::Date;
use stdweb::unstable::TryInto;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsValue(pub Value, pub i32);
js_serializable!(JsValue);
js_deserializable!(JsValue);

impl From<Block<JsValue>> for JsValue {
  fn from(block: Block<JsValue>) -> JsValue {
    let boxed = Box::new(block);
    let ptr = Box::into_raw(boxed);
    JsValue(Value::from(false), ptr as i32)
  }
}

impl From<Value> for JsValue {
  fn from(v: Value) -> Self {
    JsValue(v, 0)
  }
}

macro_rules! impl_from {
  ($type:ident) => {
    impl From<$type> for JsValue {
      fn from(it: $type) -> Self {
        JsValue(it.into(), 0)
      }
    }
  };
}

impl_from!(bool);
impl_from!(String);
impl_from!(f64);

impl From<u64> for JsValue {
  fn from(ms: u64) -> Self {
    JsValue(Value::from(Date::from_time(ms as f64).as_ref()), 0)
  }
}

impl FromIterator<(String, JsValue)> for JsValue {
  fn from_iter<I: IntoIterator<Item = (String, JsValue)>>(iterable: I) -> Self {
    let map: HashMap<_, _> = iterable.into_iter().map(|(k, v)| (k, v.0)).collect();
    JsValue(Value::from(map), 0)
  }
}

impl FromIterator<JsValue> for JsValue {
  fn from_iter<I: IntoIterator<Item = JsValue>>(iterable: I) -> Self {
    let vec: Vec<Value> = iterable.into_iter().map(|v| v.0).collect();
    JsValue(Value::Reference(Array::from(vec).into()), 0)
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

impl IValue for JsValue {
  type List = JsArray;
  type Record = JsObject;

  fn try_bool(&self) -> Result<bool, String> {
    match (*self).0 {
      Value::Bool(b) => Ok(b),
      _ => Err("not a bool".into()),
    }
  }

  fn try_string(&self) -> Result<&str, String> {
    match (*self).0 {
      Value::String(ref s) => Ok(s.as_str()),
      _ => Err("not a string".into()),
    }
  }

  fn try_number(&self) -> Result<f64, String> {
    match (*self).0 {
      Value::Number(n) => Ok(n.try_into().unwrap()),
      _ => Err("not a number".into()),
    }
  }

  fn try_time(&self) -> Result<u64, String> {
    let date: Result<Date, _> = self.0.as_ref().try_into();
    date
      .map(|d| d.get_time() as u64)
      .map_err(|err| format!("{:?}", err))
  }

  fn try_list(&self) -> Result<JsArray, String> {
    let list: Result<Array, _> = self.0.as_ref().try_into();
    // console!(log, "try_list");
    list.map(JsArray).map_err(|err| format!("{:?}", err))
  }

  fn try_record(&self) -> Result<JsObject, String> {
    let obj: Result<Object, _> = self.0.as_ref().try_into();
    obj.map(JsObject).map_err(|err| format!("{:?}", err))
  }

  fn callable(&self) -> bool {
    self.1 > 0
  }

  fn try_call(&self, args: &[Self]) -> Result<Self, String> {
    call_block_ptr(self.1, args)
  }
}

pub fn call_block_ptr(ptr: i32, args: &[JsValue]) -> Result<JsValue, String> {
  if ptr <= 0 {
    return Err("not a block".into());
  }
  let block = unsafe { Box::from_raw(ptr as *mut Block<JsValue>) };
  let result = block.call(args);
  Box::into_raw(block); // prevent drop
  result
}


//** Lists

#[derive(Debug, Clone, PartialEq)]
pub struct JsArray(Array);

impl IntoIterator for JsArray {
  type Item = JsValue;
  type IntoIter = Map<JsIter, fn(Value) -> JsValue>;

  fn into_iter(self) -> Self::IntoIter {
    // console!(log, "into_iter");
    JsIter::new(self.0.into()).map(JsValue::from)
  }
}

impl IList<JsValue> for JsArray {
  fn len(&self) -> usize {
    self.0.len()
  }

  fn at(&self, key: usize) -> Option<JsValue> {
    let value = js!{ return @{self.0.as_ref()}[@{key as i32}] };
    nil_to_none(value)
  }
}

impl IntoIterator for JsObject {
  type Item = (String, JsValue);
  type IntoIter = Map<JsIter, fn(Value) -> (String, JsValue)>;

  fn into_iter(self) -> Self::IntoIter {
    JsIter::new(self.0.into()).map(pair_to_tuple)
  }
}

fn pair_to_tuple(pair: Value) -> (String, JsValue) {
  let key = js!{ return @{&pair}[0] };
  let val = js!{ return @{&pair}[1] };
  (key.into_string().unwrap(), JsValue(val, 0))
}

//** Records

#[derive(Debug, Clone, PartialEq)]
pub struct JsObject(Object);


impl IRecord<JsValue> for JsObject {
  fn at(&self, key: &str) -> Option<JsValue> {
    let value = js!{ return @{self.0.as_ref()}[@{key}] };
    nil_to_none(value)
  }
}

fn nil_to_none(value: Value) -> Option<JsValue> {
  match value {
    Value::Undefined => None,
    Value::Null => None,
    some => Some(JsValue(some, 0)),
  }
}

/// Iterates over a `Reference` using the JS iteration protocol.
pub struct JsIter {
  iter: Value,
}

impl JsIter {
  fn new(value: Reference) -> Self {
    JsIter {
      iter: js!{ return @{value}[Symbol.iterator]() },
    }
  }
}

js_serializable!(IterNext);
js_deserializable!(IterNext);

impl Iterator for JsIter {
  type Item = Value;

  fn next(&mut self) -> Option<Value> {
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
  value: Value,
  done: bool,
}
