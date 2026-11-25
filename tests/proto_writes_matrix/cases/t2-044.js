function put(root, keys, property, value) {
  for (let i = 0; i < keys.length; i++) {
    const selector = keys[i];
    if (root[selector] == null) continue;
    root[selector][property] = value;
  }
  return root;
}

function putFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  return put(root, keys, property, value);
}

module.exports = putFromParts;
