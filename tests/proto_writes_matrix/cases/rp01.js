function merge(target, source) {
  for (const key in source) {
    if (typeof source[key] === "object") {
      if (!target[key]) target[key] = {};
      exports.merge(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  }
}

exports.merge = merge;
module.exports = merge;
