function set(root, keys, value) {
  const parts = keys.slice();
  const last = parts.pop();
  let cursor = root;
  for (const part of parts) {
    const key = String(part);
    if (key === "__proto__" || key === "constructor") return root;
    cursor = cursor[key];
  }
  cursor[last] = value;
  return root;
}

module.exports = set;
