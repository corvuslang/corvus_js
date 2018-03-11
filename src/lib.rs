#![feature(proc_macro)]

extern crate corvus_core;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate stdweb;

mod bird;

use std::collections::HashMap;
use stdweb::js_export;
use bird::Bird;
use corvus_core::{InferredEnv, Type};
use corvus_core::standalone::Value;

#[derive(Serialize, Deserialize, Debug)]
struct JsValue(Value);

#[derive(Serialize, Deserialize, Debug)]
struct JsType(Type);

#[derive(Serialize, Deserialize, Debug)]
struct TypeOf {
    ty: Type,
    inferred: InferredEnv,
}

#[derive(Serialize, Deserialize, Debug)]
enum JsResult<T> {
    Ok(T),
    Err(String),
}

type EvalResult = JsResult<Value>;
type TypeOfResult = JsResult<TypeOf>;


js_serializable!(EvalResult);
js_serializable!(TypeOf);
js_serializable!(TypeOfResult);
js_serializable!(JsValue);
js_deserializable!(JsValue);

#[js_export]
fn create() -> i32 {
    let ptr = Box::into_raw(Box::new(Bird::new()));
    ptr as i32
}

#[js_export]
fn evaluate(handle: i32, string: &str) -> JsResult<Value> {
    match with_bird(handle, |bird| bird.eval(string)) {
        Ok((val, _)) => JsResult::Ok(val),
        Err(msg) => JsResult::Err(msg),
    }
}

#[js_export]
fn type_of(handle: i32, string: &str) -> JsResult<TypeOf> {
    match with_bird(handle, |bird| bird.type_of(string)) {
        Ok((_stx, ty, inferred)) => JsResult::Ok(TypeOf {
            ty: ty,
            inferred: inferred,
        }),
        Err(msg) => JsResult::Err(msg),
    }
}

#[js_export]
fn set(handle: i32, name: &str, val: JsValue) {
    with_bird(handle, |bird| bird.set(name, val.0))
}

#[js_export]
fn vars(handle: i32) -> HashMap<String, JsValue> {
    with_bird(handle, |bird| {
        bird.vars()
            .into_iter()
            .map(|(k, v)| (k, JsValue(v.as_ref().clone())))
            .collect()
    })
}

fn with_bird<T, F: FnOnce(&mut Bird) -> T>(handle: i32, f: F) -> T {
    let mut bird: Box<Bird> = unsafe { Box::from_raw(handle as *mut Bird) };
    let result = f(&mut bird);
    Box::into_raw(bird); // don't drop the bird!
    result
}
