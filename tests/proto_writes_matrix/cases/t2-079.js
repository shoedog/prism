function updateCopy(root, section, field, value) {
  const original = root[section];
  if (original == null) return null;
  const copy = { ...original };
  copy[field] = value;
  return copy;
}

module.exports = updateCopy;
