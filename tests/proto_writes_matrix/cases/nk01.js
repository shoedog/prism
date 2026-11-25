function put(a, k1, k2, v) {
  a[k1][k2] = v;
}

function putAtPath(a, path, v) {
  const [k1, k2] = path;
  put(a, k1, k2, v);
  return a;
}

module.exports = putAtPath;
