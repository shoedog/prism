function makeWriter(root, selector, property) {
  return {
    set value(next) {
      if (root[selector] != null) {
        root[selector][property] = next;
      }
    }
  };
}

function putFromParts(root, [selector, property], value) {
  const writer = makeWriter(root, selector, property);
  writer.value = value;
  return root;
}

module.exports = putFromParts;
