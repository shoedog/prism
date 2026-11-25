function renameKey(k) {
  return ['__proto__', 'constructor', 'prototype'].includes(k) ? "_" + k : k;
}

function set(obj, path, v) {
  const keys = path.split(".").map(renameKey);
  let o = obj;
  for (const k of keys.slice(0, -1)) {
    o = o[k] = o[k] || {};
  }
  const last = keys[keys.length - 1];
  o[last] = v;
}

module.exports = set;
