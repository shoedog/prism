function put(rows, position, property, value) {
  const index = Number(position);
  if (!Number.isInteger(index) || index < 0) return rows;
  const selector = String(index);
  if (rows[selector] != null) {
    rows[selector][property] = value;
  }
  return rows;
}

function putFromParts(rows, [position, property], value) {
  return put(rows, position, property, value);
}

module.exports = putFromParts;
