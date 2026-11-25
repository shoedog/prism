function* put(root, field, value) {
  const section = yield Object.keys(root);
  if (typeof section !== "string") return root;
  if (root[section] == null) return root;
  root[section][field] = value;
  return root;
}

function putFromParts(root, parts, value) {
  const [section, field] = parts;
  const iterator = put(root, field, value);
  iterator.next();
  return iterator.next(section).value;
}

module.exports = putFromParts;
