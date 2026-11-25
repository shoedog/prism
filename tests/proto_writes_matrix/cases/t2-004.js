function set(root, path, value) {
  function* segments() {
    for (const key of path.split(".")) yield key;
  }
  const keys = Array.from(segments());
  const last = keys.pop();
  let cursor = root;
  for (const key of keys) cursor = cursor[key];
  cursor[last] = value;
  return root;
}

module.exports = set;
