const create = () =>
  Rust.corvus_js.then(corvus_js => {
    const handle = corvus_js.create();
    return {
      eval: (string) => corvus2js(unwrap(corvus_js.evaluate(handle, string))),
      typeOf: (string) => unwrap(corvus_js.type_of(handle, string)),
      set: (name, val) => corvus_js.set(handle, name, js2corvus(val)),
      vars: () => mapValues(corvus_js.vars(handle), corvus2js),
    }
  })

const unwrap = (result) => {
  if (result.Err) {
    throw new Error(result.Err)
  }
  return result.Ok
}

const js2corvus = (val) => {
  if (typeof val === 'number') {
    return {
      Prim: {
        Number: val
      }
    }
  }
  if (typeof val === 'string') {
    return {
      Prim: {
        String: val
      }
    }
  }
  if (typeof val === 'boolean') {
    return {
      Prim: {
        Boolean: val
      }
    }
  }
  if (val instanceof Date) {
    return {
      Prim: {
        Time: val.getTime()
      }
    }
  }
  if (Array.isArray(val)) {
    return {
      List: val.map(v => js2corvus(v))
    }
  }
  if (typeof val === 'object') {
    return {
      Record: mapValues(val, js2corvus)
    }
  }
}

const corvus2js = (val) => {
  if (val.Prim) {
    for (const k in val.Prim) {
      if (val.Prim.hasOwnProperty(k)) {
        return val.Prim[k]
      }
    }
  }
  if (val.Record) {
    return mapValues(val.Record, corvus2js)
  }
  if (val.List) {
    return val.List.map(corvus2js)
  }
  return val
}

const mapValues = (obj, fn) => {
  const out = {}
  for (var k in obj) {
    out[k] = fn(obj[k])
  }
  return out
}

if (typeof window !== 'undefined') {
  window.createCorvus = create
}