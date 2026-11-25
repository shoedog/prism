function put(a, k, v) {
  a["fixed"][k] = v;
  return a;
}

module.exports = put;
