function set(obj, path, v) {
  const keys = path.split(".");
  const k2 = keys.pop();
  let o = obj;
  let parent = o;
  for (const k of keys) {
    parent = o;
    o = o[k];
  }
  parent[k2] = v;
  return obj;
}

module.exports = set;
