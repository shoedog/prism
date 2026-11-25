function put(root, items, property, value) {
  for (const key in items) {
    if (root[key] != null) {
      root[key][property] = value;
    }
  }
  return root;
}

function putFromParts(root, parts, value) {
  const items = [];
  const property = parts[parts.length - 1];
  for (const part of parts.slice(0, -1)) {
    const name = String(part);
    if (name === "length") continue;
    Object.defineProperty(items, name, { value: true, enumerable: true, configurable: true });
  }
  return put(root, items, property, value);
}

module.exports = putFromParts;
