function put(root, position, names, property, value) {
  const index = Number(position);
  if (!Number.isInteger(index) || index < 0) return root;
  const selector = names[index];
  if (root[selector] != null) {
    root[selector][property] = value;
  }
  return root;
}

function putFromParts(root, position, parts, value) {
  const names = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  return put(root, position, names, property, value);
}

module.exports = putFromParts;
