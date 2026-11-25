function set(obj, path, v) {
  const keys = path.split(".");
  let o = obj;
  for (const k of keys.slice(0, -1)) {
    if (['__proto__', 'constructor', 'prototype'].includes(k)) return;
    o = o[k] = o[k] || {};
  }
  const last = keys[keys.length - 1];
  if (['__proto__', 'constructor', 'prototype'].includes(last)) return;
  o[last] = v;
}

module.exports = set;
