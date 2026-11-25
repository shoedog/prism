function labelEntry(entries, section, req, value) {
  const entry = entries[section];
  const field = req.params.field;
  entry[field] = value;
  return entries;
}

function labelPrimary(path, value) {
  return labelEntry({ a: {} }, "a", { params: { field: path[1] } }, value);
}

module.exports = labelPrimary;
