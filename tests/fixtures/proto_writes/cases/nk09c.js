function put(out, path, v) {
  const [a, k] = path;
  out[a | 0][k] = v;
  return out;
}

module.exports = put;
