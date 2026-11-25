function put(root, path, field, value) {
  if (path !== "settings.display" && path !== "profile.contact") {
    return root;
  }
  let cursor = root;
  for (const key of path.split(".")) {
    cursor = cursor[key];
    if (cursor == null) return root;
  }
  cursor[field] = value;
  return root;
}

module.exports = put;
