function put(out, path, v) {
  const [a, k] = path;
  out[+a][k] = v;
  return out;
}

module.exports = put;
