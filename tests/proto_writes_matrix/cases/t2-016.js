function put(rows, index, property, value) {
  const rowIndex = Math.floor(index);
  if (!rows[rowIndex]) return rows;
  rows[rowIndex][property] = value;
  return rows;
}

function putFromParts(rows, [index, property], value) {
  return put(rows, index, property, value);
}

module.exports = putFromParts;
