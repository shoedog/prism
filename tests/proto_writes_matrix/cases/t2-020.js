function ensure(root, selector, property, value) {
  if (root[selector] == null) return root;
  root[selector][property] ??= value;
  return root;
}

function putFromParts(root, [selector, property, value = 1]) {
  return ensure(root, selector, property, value);
}

module.exports = putFromParts;
