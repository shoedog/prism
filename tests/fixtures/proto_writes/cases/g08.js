function set(obj, path, v) {
  const keys = path.split(".");
  let o = obj;
  for (const k of keys.slice(0, -1)) {
    o = o[k] = o[k] || {};
  }
  const last = keys[keys.length - 1];
  o[last] = v;
  if (keys.some(k => k === '__proto__' || k === 'constructor' || k === 'prototype')) return;
}

module.exports = set;
