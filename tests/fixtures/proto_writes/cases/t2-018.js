function put(rows, values, property) {
  for (let i = 0; i < values.length; i++) {
    const key = String(i);
    if (!rows[key]) continue;
    rows[key][property] = values[i];
  }
}

function putFromParts(rows, [property, ...values]) {
  put(rows, values, property);
  return rows;
}

module.exports = putFromParts;
