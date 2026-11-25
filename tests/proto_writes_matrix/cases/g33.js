function put(a, k1, k2, v) {
  if (k1 === '__proto__') return;
  a[k1][k2] = v;
}

function putAtPath(a, path, v) {
  const [k1, k2] = path;
  put(a, k1, k2, v);
  return a;
}

module.exports = putAtPath;
