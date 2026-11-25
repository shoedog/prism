function set(root, keys, property, value) {
  let cursor = root;
  let i = 0;
  while (i < keys.length && (cursor = cursor[keys[i++]])) {
    if (i === keys.length) break;
  }
  if (cursor) cursor[property] = value;
  return root;
}

function setFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  return set(root, keys, property, value);
}

module.exports = setFromParts;
