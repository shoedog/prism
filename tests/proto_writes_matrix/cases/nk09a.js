function put(out, path, v) {
  const [a, k] = path;
  out[parseInt(a, 10)][k] = v;
  return out;
}

module.exports = put;
