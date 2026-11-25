function put(root, selector, property, value) {
  const write = () => (
    root[selector][property] = value
  );
  write();
  return root;
}

function putFromParts(root, [selector, property], value) {
  return put(root, selector, property, value);
}

module.exports = putFromParts;
