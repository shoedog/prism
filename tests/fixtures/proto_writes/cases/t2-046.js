function increment(root, selector, property) {
  const target = root[selector];
  if (target == null) return root;
  root[selector][property]++;
  return root;
}

function putFromParts(root, [selector, property]) {
  return increment(root, selector, property);
}

module.exports = putFromParts;
