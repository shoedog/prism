function set(obj, path, v) {
  path.split(".").forEach(function (p, i, a) {
    if (i === a.length - 1) {
      obj[p] = v;
    } else {
      obj = obj[p] = obj[p] || {};
    }
  });
}

module.exports = set;
