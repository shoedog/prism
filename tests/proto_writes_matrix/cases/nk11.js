function put(obj, path, v) {
  const [k, f] = path;
  obj['__' + k + '__'][f] = v;
  return obj;
}

module.exports = put;
