function updateCatalog(path, value) {
  const [catalog, field] = path.split(".");
  const catalogs = { a: { fields: {} } };
  if (!catalogs[catalog]) return catalogs;
  catalogs[catalog][field] = value;
  return catalogs;
}

module.exports = updateCatalog;
