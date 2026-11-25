function put(out, k, list) {
  list.forEach((x, i) => {
    out[i][k] = x;
  });
  return out;
}

module.exports = put;
