function put(rows, index, field, value) {
  if (!Number.isInteger(index)) return rows;
  if (rows[index] != null) {
    rows[index][field] = value;
  }
  return rows;
}

function putAtIndex(rows, indexText, field, value) {
  return put(rows, Number(indexText), field, value);
}

module.exports = putAtIndex;
