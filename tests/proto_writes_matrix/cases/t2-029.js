function isSafeKey(key) {
  return key !== "__proto__" &&
    key !== "constructor" && key !== "prototype";
}
function set(root, path, value) {
  const keys = path.split(".");
  const last = keys.pop();
  let cursor = root;
  for (const key of keys) {
    if (!isSafeKey(key)) return;
    cursor = cursor[key];
  }
  cursor[last] = value;
  return root;
}

module.exports = set;
