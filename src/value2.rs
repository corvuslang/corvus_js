#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
pub struct CorvusValue(Reference);

macro_rules! extract_value {
  ($me:ident, $tag:ident, $target:ty) => {
    let v = js!{ return @{$me}.$tag; };
    v.try_into::<$target>().map_err(|e| e.description().into())
  };
}

impl IValue for CorvusValue {
  fn try_bool(&self) -> Result<bool, String> {
    extract_value!(self, bool, bool)
  }

  fn try_number(&self) -> Result<f64, String> {
    extract_value!(self, number, f64)
  }

  fn try_string(&self) -> Result<&str, String> {}
}
