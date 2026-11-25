function put(rows, k, v) {
  rows[rows.length - 1][k] = v;
  return rows;
}

module.exports = put;
