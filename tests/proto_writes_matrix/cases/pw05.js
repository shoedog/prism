function set(o, keys, v) {
  if (keys.length === 1) {
    o[keys[0]] = v;
  } else {
    set(o[keys[0]] = o[keys[0]] || {}, keys.slice(1), v);
  }
}

module.exports = set;
