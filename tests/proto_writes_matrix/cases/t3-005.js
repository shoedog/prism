function labelRows(input, field) {
  const labels = [...input];
  const rows = Array.from({ length: labels.length }, () => ({}));
  for (const [index, label] of labels.entries()) {
    rows[index][field] = label;
  }
  return rows;
}

module.exports = labelRows;
