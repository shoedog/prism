function set(obj, path, value) {
  const keys = path.split(".");
  let o = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    o = o[keys[i]] = o[keys[i]] || {};
  }
  o[keys[keys.length - 1]] = value;
}

module.exports = set;
