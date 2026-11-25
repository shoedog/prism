function put(selector, property, value) {
  const rows = [{}];
  rows[selector][property] = value;
  return rows;
}

function putFromParts([selector, property], value) {
  return put(selector, property, value);
}

module.exports = putFromParts;
