function renameFilterKeys(filter) {
  for (const key in filter) {
    if (!key.includes(":")) continue;
    const [field, collection] = key.split(":");
    const normalized = `${field}:${collection}`;
    if (normalized !== key) {
      filter[normalized] = filter[key];
      delete filter[key];
    }
  }
  return filter;
}

module.exports = renameFilterKeys;
