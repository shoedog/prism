function put(root, path, value) {
  const keys = path.split(".");
  const field = keys.pop();
  let cursor = root;
  for (const key of keys) {
    cursor = (cursor[key] ??= {});
  }
  cursor[field] = value;
  return root;
}

module.exports = put;
