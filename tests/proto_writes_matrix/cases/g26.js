function set(obj, path, v) {
  const keys = Array.isArray(path) ? path : path.split(".");
  let o = obj;
  for (const k of keys.slice(0, -1)) {
    if (typeof k === 'string' && k !== '__proto__' && k !== 'constructor' && k !== 'prototype') {
      o = o[k] = o[k] || {};
    } else {
      return;
    }
  }
  const last = keys[keys.length - 1];
  o[last] = v;
}

module.exports = set;
