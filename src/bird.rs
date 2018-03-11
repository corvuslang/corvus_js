use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;

use corvus_core::{parse, type_of, Eval, InferredEnv, Namespace, ParseRule, Scope, SharedNamespace,
                  Syntax, Type};
use corvus_core::standalone::Value;

pub struct Bird {
  ns: SharedNamespace<Value>,
  values: RefCell<Scope<Value>>,
  types: RefCell<Scope<Type>>,
}

impl Bird {
  pub fn new() -> Bird {
    Bird {
      ns: Namespace::new_with_prelude().unwrap().into_shared(),
      values: RefCell::new(Scope::new()),
      types: RefCell::new(Scope::new()),
    }
  }

  fn parse(&self, input: &str) -> Result<Syntax, String> {
    parse(&*self.ns.borrow(), ParseRule::term, input).map_err(|e| format!("{}", e))
  }

  pub fn type_of(&self, input: &str) -> Result<(Syntax, Type, InferredEnv), String> {
    self.parse(input).and_then(|stx| {
      let types = self.types.borrow().flatten();
      let global_types = types
        .iter()
        .map(|(name, ty)| (name.clone(), ty.as_ref().clone()));
      match type_of(&*self.ns.borrow(), global_types, &stx) {
        Ok((ty, mut inferred_env)) => {
          self.filter_env(&mut inferred_env);
          Ok((stx.clone(), ty, inferred_env))
        }
        Err(type_errors) => Err(type_errors),
      }
    })
  }

  pub fn eval(&self, input: &str) -> Result<(Value, Type), String> {
    self.type_of(input).and_then(|(stx, ty, _inferred_env)| {
      let values = self.values.borrow();
      stx
        .eval(&self.ns, &values)
        .map(move |val| (val, ty))
        .map_err(|err| format!("runtime error: {}", err))
    })
  }

  pub fn set(&self, name: &str, val: Value) {
    self
      .types
      .borrow_mut()
      .insert(String::from(name), val.type_of());
    self.values.borrow_mut().insert(String::from(name), val);
  }

  pub fn vars(&self) -> HashMap<String, Rc<Value>> {
    self.values.borrow().flatten()
  }

  fn filter_env(&self, env: &mut InferredEnv) {
    let globals = self.types.borrow();
    env.retain(|k, _v| globals.get(k).is_none());
  }
}
