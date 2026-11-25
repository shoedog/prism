function combineCounts(order, field = "count") {
  const totals = Object.assign(Object.create(null), { count: 0 });
  const reports = [Object.create(totals), Object.create(totals)];
  const latest = { count: 1 };
  const buckets = Object.assign(Object.create(null), { team: totals, latest });
  const items = order.split(",").map(key => buckets[key]).filter(Boolean);
  if (items.length >= 2) {
    items.reduce((first, item) => {
      first[field] += item[field];
      return first;
    });
  }
  return { reports, latest };
}

module.exports = combineCounts;
