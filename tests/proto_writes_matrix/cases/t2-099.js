function put(rows, offset, field, value) {
  const row = rows.at(offset);
  if (row == null) return rows;
  row[field] = value;
  return rows;
}

module.exports = put;
