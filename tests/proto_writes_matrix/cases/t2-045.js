function put(rows, index, property, value) {
  if (rows[index] == null) return rows;
  if (index >= 0) {
    rows[index][property] = value;
  }
  return rows;
}

function putFromParts(rows, [index, property], value) {
  return put(rows, index, property, value);
}

module.exports = putFromParts;
