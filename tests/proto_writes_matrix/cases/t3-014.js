function enableEntry(entries, name, req) {
  let requested;
  {
    const name = req.params.name;
    requested = name;
  }
  const entry = entries[name];
  entry.enabled = true;
  return { entries, requested };
}

function enablePrimary(path) {
  return enableEntry({ a: {} }, "a", { params: { name: path[1] } });
}

module.exports = enablePrimary;
