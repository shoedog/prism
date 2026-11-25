function set(obj, path, v) {
  const keys = path.split(".");
  let o = obj;
  for (const k of keys.slice(0, -1)) {
    if (k !== '__proto__' && k !== 'constructor' && k !== 'prototype') {
      o = o[k] = o[k] || {};
    }
  }
  const last = keys[keys.length - 1];
  o[last] = v;
}

module.exports = set;
