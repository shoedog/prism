function labelRows(entries, field) {
  const rows = { first: {}, second: {} };
  const labels = {
    entries() {
      return Object.keys(entries).map((name) => [name, entries[name]]);
    }
  };
  for (const [index, label] of labels.entries()) {
    rows[index][field] = label;
  }
  return rows;
}

module.exports = labelRows;
