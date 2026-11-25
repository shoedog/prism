function resetCounts(scope, fields = ["count"]) {
  const totals = Object.assign(Object.create(null), { count: 1 });
  const reports = [Object.create(totals), Object.create(totals)];
  const scopes = Object.assign(Object.create(null), { team: totals, first: reports[0] });
  const selected = scopes[scope];
  if (selected !== undefined) {
    fields.forEach(function (field) { this[field] = 0; }, selected);
  }
  return reports;
}

module.exports = resetCounts;
