function labelRows(input, field) {
  const labels = [...input];
  const rows = Array.from({ length: labels.length }, () => ({}));
  labels.forEach((label, index) => {
    rows[index][field] = label;
  });
  return rows;
}

module.exports = labelRows;
