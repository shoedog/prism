function walk(o, keys) {
  for (const k of keys) {
    o = o[k] = o[k] || {};
  }
  return o;
}

function set(obj, path, v) {
  const keys = path.split(".");
  const last = keys.pop();
  const parent = walk(obj, keys);
  parent[last] = v;
}

module.exports = set;
