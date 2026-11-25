function define(root, selector, property, value) {
  const base = root[selector];
  if (base == null) return root;
  Reflect.defineProperty(base, property, {
    value,
    writable: true,
    configurable: true
  });
  return root;
}

function defineFromParts(root, parts, value) {
  const [selector, property] = parts;
  return define(root, selector, property, value);
}

module.exports = defineFromParts;
