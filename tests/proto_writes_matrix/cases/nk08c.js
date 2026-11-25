function put(out, k, list, x) {
  out[list.indexOf(x)][k] = x;
  return out;
}

module.exports = put;
