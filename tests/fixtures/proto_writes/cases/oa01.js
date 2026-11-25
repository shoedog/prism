function merge(target, source) {
  for (const key of Object.keys(source)) {
    if (source[key] && typeof source[key] === "object") {
      if (!target[key]) target[key] = {};
      if (Object.values(source[key]).every(value => value === null || typeof value !== "object")) {
        Object.assign(target[key], source[key]);
      } else {
        merge(target[key], source[key]);
      }
    } else {
      target[key] = source[key];
    }
  }
}

module.exports = merge;
