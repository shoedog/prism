function enableEntry(entries, req) {
  const section = req.params.section;
  const entry = entries[section];
  entry.enabled = true;
  return entries;
}

function enablePrimary(path) {
  return enableEntry({ a: {} }, { params: { section: path[0] } });
}

module.exports = enablePrimary;
