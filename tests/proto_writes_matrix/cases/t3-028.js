function mergeFilter(target, patch) {
  for (const key in patch) {
    if (patch[key] !== null && typeof patch[key] === "object" && !Array.isArray(patch[key])) {
      if (!target[key]) target[key] = {};
      mergeFilter(target[key], patch[key]);
    } else if (typeof patch[key] === "string") {
      target[key] = patch[key].trim();
    } else {
      target[key] = patch[key];
    }
  }
  return target;
}

module.exports = mergeFilter;
