function set(root, path, value) {
  const keys = path.split(".");
  const last = keys.pop();
  let cursor = root;
  keys.map(key => {
    cursor = cursor[key];
    return cursor;
  });
  cursor[last] = value;
  return root;
}

module.exports = set;
