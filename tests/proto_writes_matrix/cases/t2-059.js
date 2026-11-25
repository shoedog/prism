function put(root, selector, property, input, fallback) {
  if (root[selector] == null) return root;
  ({
    value: root[selector][property] = fallback
  } = input);
  return root;
}

function putFromParts(root, [selector, property, value], fallback) {
  return put(root, selector, property, { value }, fallback);
}

module.exports = putFromParts;
