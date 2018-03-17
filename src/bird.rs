use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;

use corvus_core::{parse, type_of, Eval, Namespace, ParseRule, Scope, SharedNamespace, Syntax,
                  Type, TypeCheckerResult};

use value::JsValue as Value;

pub struct Bird {
  pub ns: SharedNamespace<Value>,
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

  pub fn type_of(&self, input: &str) -> Result<(Syntax, TypeCheckerResult), String> {
    self.parse(input).map(|stx| {
      let types = self.types.borrow().flatten();
      let global_types = types
        .iter()
        .map(|(name, ty)| (name.clone(), ty.as_ref().clone()));
      let result = type_of(&*self.ns.borrow(), global_types, &stx);
      (stx, result)
    })
  }

  pub fn eval(&self, code: &str, inputs: HashMap<String, Value>) -> Result<Value, String> {
    self.parse(code).and_then(|stx| {
      let mut scope = self.values.borrow().new_child_with_capacity(inputs.len());
      for (key, val) in inputs {
        scope.insert(key, val);
      }
      stx
        .eval(&self.ns, &scope)
        .map_err(|err| format!("runtime error: {}", err))
    })
  }

  pub fn set(&self, name: &str, val: Value) {
    self
      .types
      .borrow_mut()
      .insert(String::from(name), Type::Any);
    self.values.borrow_mut().insert(String::from(name), val);
  }

  pub fn vars(&self) -> HashMap<String, Rc<Value>> {
    self.values.borrow().flatten()
  }
}
