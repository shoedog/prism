function setIn(o, path, last, v) {
  for (const k of path) {
    o = o[k] = o[k] || {};
  }
  o[last] = v;
}

module.exports = setIn;
