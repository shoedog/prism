function merge(target, source) {
  const descend = merge.bind(null);
  for (const key of Object.keys(source)) {
    const value = source[key];
    if (value && typeof value === "object") {
      if (!target[key]) target[key] = {};
      descend(target[key], value);
    } else {
      target[key] = value;
    }
  }
}

module.exports = merge;
