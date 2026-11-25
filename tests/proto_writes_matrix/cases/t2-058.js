function put(root, names, sought, fallback, property, value) {
  let selector = names.indexOf(sought);
  if (selector < 0) selector = fallback;
  if (root[selector] != null) {
    root[selector][property] = value;
  }
  return root;
}

function putFromParts(root, names, sought, [fallback, property], value) {
  return put(root, names, sought, fallback, property, value);
}

module.exports = putFromParts;
