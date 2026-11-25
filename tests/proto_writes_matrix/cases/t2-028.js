function set(root, parentPath, property, value) {
  if (parentPath === "__proto__" || parentPath === "constructor") return;
  let cursor = root;
  for (const key of parentPath.split(".")) {
    cursor = cursor[key];
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
