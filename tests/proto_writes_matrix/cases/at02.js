function set(out, path, v) {
  const [section, key] = path;
  let p;
  p = out[section] = out[section] || {};
  p[key] = v;
}

module.exports = set;
