function annotateReports(keys, field, value) {
  const defaults = Object.create(null);
  const reports = [Object.create(defaults), Object.create(defaults)];
  let candidates = [{ summary: {} }, { summary: defaults }];
  for (const key of keys) {
    candidates = candidates.flatMap(node =>
      Object.hasOwn(node, key) ? [node[key]] : []
    );
  }
  for (const node of candidates) node[field] = value;
  return reports;
}

module.exports = annotateReports;
