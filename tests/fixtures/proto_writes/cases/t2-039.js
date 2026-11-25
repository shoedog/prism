function set(root, path, value) {
  const keys = path.split(".");
  const last = keys.pop();
  let cursor = root;
  const reached = keys.every(key => {
    cursor = cursor[key];
    return cursor != null;
  });
  if (reached) cursor[last] = value;
  return root;
}

module.exports = set;
