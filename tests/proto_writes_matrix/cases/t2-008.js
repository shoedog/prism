function set(root, keys, value) {
  let cursor = root;
  function advance(key) {
    cursor = cursor[key];
  }
  const parts = keys.slice();
  const last = parts.pop();
  for (const key of parts) advance(key);
  cursor[last] = value;
  return root;
}

module.exports = set;
