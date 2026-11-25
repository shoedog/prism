function labelEntry(entries, section, req) {
  const entry = entries[section];
  const value = req.params.label;
  entry.label = value;
  return entries;
}

function labelPrimary(path) {
  return labelEntry({ a: {} }, "a", { params: { label: path[1] } });
}

module.exports = labelPrimary;
