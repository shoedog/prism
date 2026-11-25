function merge(target, source) {
  Object.keys(source).forEach(key => {
    if (typeof source[key] === "object") {
      if (!target[key]) target[key] = {};
      merge(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  });
}

module.exports = merge;
