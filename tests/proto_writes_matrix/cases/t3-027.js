function normalizeFilter(filter) {
  for (const key in filter) {
    if (Array.isArray(filter[key])) {
      filter[key] = filter[key].map(String);
    } else if (typeof filter[key] === "string") {
      filter[key] = filter[key].trim();
    }
  }
  return filter;
}

module.exports = normalizeFilter;
