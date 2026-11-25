function set(root, parts, value) {
  const keys = parts.slice();
  const last = keys.pop();
  let cursor = root;
  do {
    const key = keys.pop();
    cursor = cursor[key];
  } while (keys.length);
  cursor[last] = value;
  return root;
}

module.exports = set;
