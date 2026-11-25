function put(out, k, list) {
  for (const [i, x] of list.entries()) {
    out[i][k] = x;
  }
  return out;
}

module.exports = put;
