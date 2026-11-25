function set(root, keys, value) {
  const parts = keys.slice();
  const last = parts.pop();
  let cursor = root;
  for (const key of parts) {
    [cursor] = [cursor[key]];
  }
  cursor[last] = value;
  return root;
}

module.exports = set;
