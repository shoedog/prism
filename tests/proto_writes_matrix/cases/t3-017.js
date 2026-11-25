function activateEntry(req, section, value) {
  const entries = req.body;
  const entry = entries[section];
  if (entry !== null && typeof entry === "object") {
    entry.active = value;
  }
  return entries;
}

function activatePrimary(entries, value) {
  return activateEntry({ body: entries }, "a", value);
}

module.exports = activatePrimary;
