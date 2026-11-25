function add(root, selector, property, amount) {
  const base = root[selector];
  if (base == null) return;
  root[selector][property] += amount;
  return root;
}

function putFromParts(root, [selector, property, amount = 1]) {
  return add(root, selector, property, amount);
}

module.exports = putFromParts;
