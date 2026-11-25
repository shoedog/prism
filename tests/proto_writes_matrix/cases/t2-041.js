function put(root, { selector, property }, value) {
  const selected = root[selector];
  if (selected == null) return root;
  selected[property] = value;
  return root;
}

function putFromParts(root, [selector, property], value) {
  return put(root, { selector, property }, value);
}

module.exports = putFromParts;
