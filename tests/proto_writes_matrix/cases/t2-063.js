function put(root, keys, property, value) {
  const first = String(keys[0]);
  if (first === "__proto__" || first === "constructor") return root;
  const second = keys[1];
  if (root[first] != null && root[first][second] != null) {
    root[first][second][property] = value;
  }
  return root;
}

function putFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  return put(root, keys, property, value);
}

module.exports = putFromParts;
