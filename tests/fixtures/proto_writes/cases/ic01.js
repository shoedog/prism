function set(obj, path, v) {
  const keys = path.split(".");
  const last = keys.pop();
  let o = obj;
  for (const k of keys) {
    const next = o[k];
    o = next;
  }
  o[last] = v;
  return obj;
}

module.exports = set;
