function put(rows, input, property, value) {
  const index = input.length - 1;
  if (index < 0) return rows;
  const row = rows[index];
  if (row != null) {
    rows[index][property] = value;
  }
  return rows;
}

module.exports = put;
