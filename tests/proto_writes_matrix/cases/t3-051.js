function activateEntry(req, value) {
  const entries = req.body.entries;
  const entry = entries[req.params.section];
  if (entry !== null && typeof entry === "object") {
    entry.active = value;
  }
  return entries;
}

function activatePrimary(path, value) {
  return activateEntry({ body: { entries: { a: {} } }, params: { section: path[0] } }, value);
}

module.exports = activatePrimary;
