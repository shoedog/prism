function put(root, input, property, value) {
  const names = Object.getOwnPropertyNames(input);
  for (const name of names) {
    if (root[name] == null) continue;
    root[name][property] = value;
  }
  return root;
}

function putFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  const input = Object.fromEntries(keys.map(key => [key, true]));
  return put(root, input, property, value);
}

module.exports = putFromParts;
