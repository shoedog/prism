function set(obj, parts, v) {
  while (parts.length > 1) {
    const k = parts.shift();
    obj = obj[k] = obj[k] || {};
  }
  obj[parts[0]] = v;
}

module.exports = set;
