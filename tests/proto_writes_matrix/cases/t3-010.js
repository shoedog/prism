function updateCatalog(path, value) {
  const [catalog, field] = path.split(".");
  const catalogs = { a: { fields: {} } };
  if (!catalogs[catalog]?.fields) return catalogs;
  catalogs[catalog].fields[field] = value;
  return catalogs;
}

module.exports = updateCatalog;
