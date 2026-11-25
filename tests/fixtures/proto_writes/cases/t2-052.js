function set(root, keys, property, value) {
  let cursor = root;
  for (const key of keys) {
    try {
      if (cursor == null) return root;
    } finally {
      cursor = cursor[key];
    }
  }
  if (cursor != null) cursor[property] = value;
  return root;
}

function setFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  return set(root, keys, property, value);
}

module.exports = setFromParts;
