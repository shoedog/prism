function normalizeCounts(counts) {
  for (const key of Object.keys(counts)) {
    const value = counts[key];
    if (value !== null && typeof value === "object") {
      normalizeCounts(counts[key]);
    } else if (typeof value === "number") {
      counts[key] = counts[key] + 1;
    }
  }
  return counts;
}

module.exports = normalizeCounts;
