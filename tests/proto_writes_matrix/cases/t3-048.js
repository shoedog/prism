const KEY = "enabled";

function setEntry(entries, req, value) {
  const section = req.params.section;
  const entry = entries[section];
  entry[KEY] = value;
  return entries;
}

function setPrimary(path, value) {
  return setEntry({ a: {} }, { params: { section: path[0] } }, value);
}

module.exports = setPrimary;
