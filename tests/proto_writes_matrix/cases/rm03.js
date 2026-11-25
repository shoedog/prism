function mergeObject(target, source) {
  for (const key in source) {
    if (typeof source[key] === "object") {
      if (!target[key]) target[key] = {};
      mergeValue(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  }
}

function mergeValue(target, value) {
  mergeObject(target, value);
}

module.exports = mergeObject;
