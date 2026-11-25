function set(root, path, property, value) {
  let cursor = root;
  for (const key of path.split(".")) {
    if (key === "__proto__") continue;
    cursor = cursor[key];
    if (cursor == null) return root;
  }
  cursor[property] = value;
  return root;
}

function setFromPath(root, path, value) {
  const keys = path.split(".");
  const property = keys.pop();
  return set(root, keys.join("."), property, value);
}

module.exports = setFromPath;
