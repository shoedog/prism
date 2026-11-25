function resetSummary(scope, fields = ["count"]) {
  const totals = Object.assign(Object.create(null), { count: 1 });
  const reports = [Object.create(totals), Object.create(totals)];
  const scopes = Object.assign(Object.create(null), { team: totals, first: reports[0] });
  const summary = {
    count: 1,
    reset() {
      const selected = scopes[scope];
      if (selected !== undefined) {
        fields.forEach(field => { this[field] = 0; }, selected);
      }
    }
  };
  summary.reset();
  return { summary, reports };
}

module.exports = resetSummary;
