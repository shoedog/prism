function put(entries, keys, v) {
  const map = new Map(entries);
  const [k1, k2] = keys;
  map.get(k1)[k2] = v;
  return map;
}

module.exports = put;
