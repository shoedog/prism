function put(root, checkedKey, replacement, property, value) {
  let selector = checkedKey;
  if (typeof selector !== "string") return root;
  if (selector === "__proto__" || selector === "constructor") return root;
  selector = replacement;
  if (root[selector] != null) {
    root[selector][property] = value;
  }
  return root;
}

function putFromParts(root, checkedKey, parts, value) {
  const [replacement, property] = parts;
  return put(root, checkedKey, replacement, property, value);
}

module.exports = putFromParts;
