function set(obj, path, v) {
  const keys = path.split(".");
  const last = keys.pop();
  let cursor = obj;
  for (const k of keys) {
    cursor = cursor[k] = cursor[k] || {};
  }
  Object.defineProperty(cursor, last, { value: v });
}

module.exports = set;
