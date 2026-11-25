function unflatten(entries) {
  const result = {};
  for (const [path, value] of Object.entries(entries)) {
    const keys = path.split(".");
    const last = keys.pop();
    let o = result;
    for (const k of keys) {
      o = o[k] = o[k] || {};
    }
    o[last] = value;
  }
  return result;
}

module.exports = unflatten;
