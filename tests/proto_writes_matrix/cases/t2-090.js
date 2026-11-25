function put(root, section, field, value, useProfile) {
  if (useProfile) section = "profile";
  const key = arguments[1];
  if (root[key] == null) return root;
  root[key][field] = value;
  return root;
}

function putAtPath(root, path, value, useProfile) {
  const [section, field] = path;
  return put(root, section, field, value, useProfile);
}

module.exports = putAtPath;
