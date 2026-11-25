function assignAt(root, keys, source) {
  let cursor = root;
  for (const key of keys) {
    cursor = cursor[key];
  }
  Object.assign(cursor, source);
  return root;
}

function assignFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  return assignAt(root, keys, { [property]: value });
}

module.exports = assignFromParts;
