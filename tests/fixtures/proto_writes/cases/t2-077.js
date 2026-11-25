function applyValues(root, section, field, values) {
  let count = 0;
  for (root[section][field] of values) {
    count++;
  }
  return count;
}

function applyFromParts(root, parts, value) {
  const [section, field] = parts;
  applyValues(root, section, field, [value]);
  return root;
}

module.exports = applyFromParts;
