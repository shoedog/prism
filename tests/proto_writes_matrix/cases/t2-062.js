function put(root, selector, property, value) {
  try {
    if (root == null) throw new Error("missing root");
  } catch (error) {
    if (selector === "__proto__" || selector === "constructor") return root;
  }
  if (root != null && root[selector] != null) {
    root[selector][property] = value;
  }
  return root;
}

function putFromParts(root, parts, value) {
  const [selector, property] = parts;
  return put(root, selector, property, value);
}

module.exports = putFromParts;
