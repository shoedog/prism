function clone(source) {
  if (source === null || typeof source !== "object") return source;
  const copy = Array.isArray(source) ? [] : {};
  for (const [key, value] of Object.entries(source)) {
    copy[key] = clone(value);
  }
  return copy;
}

module.exports = clone;
