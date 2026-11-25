function set(root, path, value) {
  const keys = path.split(".");
  const last = keys.pop();
  let cursor = root;
  for (let key of keys) {
    if (key === "__proto__" || key === "constructor") return;
    key = decodeURIComponent(key);
    cursor = cursor[key];
  }
  cursor[last] = value;
  return root;
}

module.exports = set;
