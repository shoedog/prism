function applyCounts(update) {
  const counts = { orders: { open: 0, closed: 0 } };
  function visit(target, source) {
    for (const key of Object.keys(target)) {
      if (!Object.hasOwn(source, key)) continue;
      const incoming = source[key];
      if (typeof target[key] === "object") {
        if (incoming && typeof incoming === "object") visit(target[key], incoming);
      } else if (typeof incoming === "number") {
        target[key] = incoming;
      }
    }
  }
  visit(counts, update);
  return counts;
}

module.exports = applyCounts;
