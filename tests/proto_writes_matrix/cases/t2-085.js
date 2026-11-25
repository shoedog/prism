function put(root, keys, field, value) {
  let cursor = root;
  for (const key of keys) {
    cursor = Reflect.get(cursor, key);
    if (cursor == null) return root;
  }
  cursor[field] = value;
  return root;
}

function putFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const field = parts[parts.length - 1];
  return put(root, keys, field, value);
}

module.exports = putFromParts;
