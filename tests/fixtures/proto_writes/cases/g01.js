function merge(target, source) {
  for (const key in source) {
    if (key === '__proto__' || key === 'constructor') continue;
    if (typeof source[key] === "object") {
      if (!target[key]) target[key] = {};
      merge(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  }
}

module.exports = merge;
