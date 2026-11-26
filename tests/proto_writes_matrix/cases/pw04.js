function set(obj, keys, v) {
  keys.slice(0, -1).reduce((o, k) => (o[k] = o[k] || {}), obj)[keys[keys.length - 1]] = v;
}

module.exports = set;
