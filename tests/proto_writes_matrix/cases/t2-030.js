async function put(root, selector, property, valuePromise) {
  const value = await valuePromise;
  if (root[selector] == null) return root;
  root[selector][property] = value;
  return root;
}

function putFromParts(root, [selector, property], value) {
  return put(root, selector, property, Promise.resolve(value));
}

module.exports = putFromParts;
