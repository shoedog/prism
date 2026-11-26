function put(a, path, v) {
  const [k1, k2] = path;
  Reflect.set(a[k1], k2, v);
  return a;
}

module.exports = put;
