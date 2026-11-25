function merge(target, source) {
  for (const key in source) {
    if (typeof source[key] === "object") {
      if (!(Object.hasOwn(target, key) &&
            Object.prototype.propertyIsEnumerable.call(target, key))) {
        target[key] = {};
      }
      merge(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  }
}

module.exports = merge;
