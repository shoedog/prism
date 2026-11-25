function set(obj, path, v) {
  const keys = path.split(".").filter(k => !['__proto__', 'constructor', 'prototype'].includes(k));
  let o = obj;
  for (const k of keys.slice(0, -1)) {
    o = o[k] = o[k] || {};
  }
  const last = keys[keys.length - 1];
  o[last] = v;
}

module.exports = set;
