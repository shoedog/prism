function set(root, value, ...keys) {
  const last = keys.pop();
  let cursor = root;
  for (const key of keys) {
    cursor = cursor[key];
    if (cursor == null) return root;
  }
  cursor[last] = value;
  return root;
}

function setFromParts(root, keys, value) {
  return set(root, value, ...keys);
}

module.exports = setFromParts;
