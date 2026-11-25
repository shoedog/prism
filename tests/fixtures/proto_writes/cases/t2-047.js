function collect(root, selector, property, values) {
  if (root[selector] == null) return root;
  let first;
  [first, ...root[selector][property]] = values;
  return root;
}

function putFromParts(root, [selector, property, ...values]) {
  return collect(root, selector, property, values);
}

module.exports = putFromParts;
