function labelRows(entries, field) {
  const rows = { first: {}, second: {} };
  const labels = {
    forEach(callback) {
      for (const name of Object.keys(entries)) callback(entries[name], name);
    }
  };
  labels.forEach((label, index) => {
    rows[index][field] = label;
  });
  return rows;
}

module.exports = labelRows;
