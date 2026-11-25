function set(root, path, property, value) {
  const allowed = new Set(["settings", "profile", "display"]);
  let cursor = root;
  for (const key of path.split(".")) {
    if (!allowed.has(key)) return root;
    cursor = cursor[key];
    if (cursor == null) return root;
  }
  cursor[property] = value;
  return root;
}

module.exports = set;
