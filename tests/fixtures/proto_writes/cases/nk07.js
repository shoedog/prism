function fill(rows, col, v) {
  for (let i = 0; i < rows.length; i++) {
    const row = rows[i];
    row[col] = v;
  }
  return rows;
}

module.exports = fill;
