function put(root, selector, property, input) {
  if (root[selector] == null) return root;
  ({ value: root[selector][property] } = input);
  return root;
}

function putFromParts(root, [selector, property, value = 1]) {
  return put(root, selector, property, { value });
}

module.exports = putFromParts;
