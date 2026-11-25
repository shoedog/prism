function accumulateCounts(target, source) {
  for (const key of Object.keys(source)) {
    const value = source[key];
    if (value !== null && typeof value === "object") {
      if (!target[key]) target[key] = {};
      accumulateCounts(target[key], value);
    } else {
      target[key] = (target[key] || 0) + value;
    }
  }
  return target;
}

module.exports = accumulateCounts;
