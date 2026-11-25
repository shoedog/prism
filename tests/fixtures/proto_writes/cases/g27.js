function set(obj, path, v) {
  const keys = path.split(".");
  let o = obj;
  for (const k of keys.slice(0, -1)) {
    o = o[k] = o[k] || {};
  }
  const last = keys[keys.length - 1];
  Object.defineProperty(o, last, { value: v, enumerable: true, writable: true, configurable: true });
}

module.exports = set;
