function isSafeKey(k) {
  return k !== '__proto__' && k !== 'constructor' && k !== 'prototype';
}

function set(obj, path, v) {
  const keys = path.split(".");
  let o = obj;
  for (const k of keys.slice(0, -1)) {
    if (!isSafeKey(k)) return;
    o = o[k] = o[k] || {};
  }
  const last = keys[keys.length - 1];
  o[last] = v;
}

module.exports = set;
