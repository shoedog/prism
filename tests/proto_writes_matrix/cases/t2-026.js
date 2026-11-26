function set(root, path, value) {
  const keys = path.split(".");
  const last = keys.pop();
  const first = keys.shift();
  if (first === "__proto__" || first === "constructor") return;
  let cursor = root[first];
  for (const key of keys) cursor = cursor[key];
  cursor[last] = value;
  return root;
}

module.exports = set;
