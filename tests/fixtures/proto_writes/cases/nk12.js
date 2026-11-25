function put(a, c, k, v) {
  a[c ? 'x' : 'y'][k] = v;
  return a;
}

module.exports = put;
