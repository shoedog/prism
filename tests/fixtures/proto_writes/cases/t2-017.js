function put(rows, key, property, value) {
  let index = 0;
  for (; index < 1; index++) {
    index = key;
    rows[index][property] = value;
    break;
  }
}

function putFromParts(rows, [key, property], value) {
  put(rows, key, property, value);
  return rows;
}

module.exports = putFromParts;
