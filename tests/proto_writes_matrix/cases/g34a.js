function set(obj, path, v) {
  const keys = path.split(".");
  let o = obj;
  for (let k of keys.slice(0, -1)) {
    if (['__proto__', 'constructor', 'prototype'].includes(k)) return;
    k = k.toLowerCase();
    o = o[k] = o[k] || {};
  }
  const last = keys[keys.length - 1];
  o[last] = v;
}

module.exports = set;
