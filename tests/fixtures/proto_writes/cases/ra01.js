function extend(deep, target, source) {
  for (const name in source) {
    const copy = source[name];
    if (deep && copy && typeof copy === "object") {
      let clone = target[name];
      if (!clone) {
        clone = {};
        target[name] = clone;
      }
      extend(deep, clone, copy);
    } else {
      target[name] = copy;
    }
  }
}

module.exports = extend;
