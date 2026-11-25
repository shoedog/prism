function put(root, selector, property, value) {
  const base = root[selector];
  if (base == null) return root;
  const patch = {};
  Object.defineProperty(patch, property, {
    value,
    enumerable: true
  });
  Object.assign(base, patch);
  return root;
}

function putFromParts(root, parts, value) {
  const [selector, property] = parts;
  return put(root, selector, property, value);
}

module.exports = putFromParts;
