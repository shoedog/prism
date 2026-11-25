function setPath(obj, keys, v) {
  if (keys.length === 0) return v;
  const [key, ...rest] = keys;
  obj = obj || {};
  obj[key] = setPath(obj[key], rest, v);
  return obj;
}

module.exports = setPath;
