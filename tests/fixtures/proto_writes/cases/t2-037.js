function set(root, path, value) {
  const keys = path.split(".");
  const last = keys.pop();
  let cursor = root;
  for (let i = 0; i < keys.length; cursor = cursor[keys[i++]]) {
    if (cursor == null) return root;
  }
  cursor[last] = value;
  return root;
}

module.exports = set;
