#![feature(proc_macro)]

extern crate corvus_core;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate stdweb;

mod value;
mod bird;

use stdweb::unstable::TryInto;
use std::collections::HashMap;
use stdweb::js_export;

use corvus_core::{Apply, Type, TypeCheckerResult};
use corvus_core::signature::Signature;

use bird::Bird;
use value::JsValue;

#[derive(Serialize, Deserialize, Debug)]
struct JsApply(Vec<(String, JsValue)>);

#[derive(Serialize, Deserialize, Debug)]
struct JsType(Type);

#[derive(Serialize, Deserialize, Debug)]
enum JsResult<T> {
    Ok(T),
    Err(String),
}

impl<O, E> From<Result<O, E>> for JsResult<O>
where
    E: std::fmt::Display,
{
    fn from(result: Result<O, E>) -> Self {
        match result {
            Ok(ok) => JsResult::Ok(ok),
            Err(err) => JsResult::Err(format!("{}", err)),
        }
    }
}


#[derive(Debug, Deserialize, Serialize)]
pub struct JsSignature(Signature);


type EvalResult = JsResult<JsValue>;
type TypeOfResult = JsResult<TypeCheckerResult>;
type VoidResult = JsResult<()>;

js_serializable!(VoidResult);
js_serializable!(EvalResult);
js_deserializable!(EvalResult);
js_serializable!(TypeOfResult);
js_serializable!(JsApply);
js_deserializable!(JsApply);
js_serializable!(JsSignature);
js_deserializable!(JsSignature);

#[js_export]
fn alloc_bird() -> i32 {
    let bird = Bird::new();
    let boxed_bird = Box::new(bird);
    let pointy_bird = Box::into_raw(boxed_bird);
    pointy_bird as i32
}

#[js_export]
fn drop_bird(handle: i32) {
    let _bird: Box<Bird> = unsafe { Box::from_raw(handle as *mut Bird) };
    // let rust drop the box
}

#[js_export]
fn evaluate(handle: i32, code: &str, inputs: HashMap<String, JsValue>) -> JsResult<JsValue> {
    with_bird(handle, |bird| bird.eval(code, inputs)).into()
}

#[js_export]
fn type_of(handle: i32, code: &str) -> JsResult<TypeCheckerResult> {
    with_bird(handle, |bird| bird.type_of(code))
        .map(|(_stx, type_info)| type_info)
        .into()
}

#[js_export]
fn set_var(handle: i32, name: &str, val: JsValue) {
    with_bird(handle, |bird| bird.set(name, val));
}

#[js_export]
fn get_var(handle: i32) -> HashMap<String, JsValue> {
    with_bird(handle, |bird| {
        bird.vars()
            .into_iter()
            .map(|(k, v)| (k, v.as_ref().clone()))
            .collect()
    })
}

#[js_export]
fn define(handle: i32, signature: JsSignature, callback: ::stdweb::Object) -> JsResult<()> {
    with_bird(handle, |bird| {
        bird.ns
            .borrow_mut()
            .insert(signature.0, wrap_callback(callback))
            .into()
    })
}

#[js_export]
fn call_block(block_ptr: i32, args: &[JsValue]) -> EvalResult {
    // console!(log, "call block", block_ptr, format!("{:?}", args));
    value::call_block_ptr(block_ptr, args).into()
}

#[js_export]
fn drop_block(block_ptr: i32) {
    use corvus_core::Block;
    let _b = unsafe { Box::from_raw(block_ptr as *mut Block<JsValue>) };
    // console!(log, "dropping block", format!("{:?}", _b));
}

fn wrap_callback(callback: ::stdweb::Object) -> Box<Fn(Apply<JsValue>) -> Result<JsValue, String>> {
    Box::new(move |args| {
        let list: Vec<_> = args.into_iter().collect();
        let result: Result<EvalResult, ::stdweb::serde::ConversionError> = js!{
            var cb = @{callback.clone()};
            return cb.call(null, @{JsApply(list)})
        }.try_into();

        result
            .map_err(|err| format!("{:?}", err))
            .and_then(|val| match val {
                JsResult::Ok(val) => Ok(val),
                JsResult::Err(msg) => Err(msg),
            })
    })
}

fn with_bird<T, F: FnOnce(&mut Bird) -> T>(handle: i32, f: F) -> T {
    let mut bird: Box<Bird> = unsafe { Box::from_raw(handle as *mut Bird) };
    let result = f(&mut bird);
    Box::into_raw(bird); // don't drop the bird!
    result
}
