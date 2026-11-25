function put(root, seen, label, field, value) {
  if (root.settings == null) return root;
  root[(seen.push(label), "settings")][field] = value;
  return root;
}

function recordLabel(root, label, field, value) {
  const seen = [];
  return { root: put(root, seen, label, field, value), seen };
}

module.exports = recordLabel;
