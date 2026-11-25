function mergePatch(target, source) {
  for (const key of Object.keys(source)) {
    const value = source[key];
    if (value && typeof value === "object" && target[key]) {
      Reflect.apply(mergePatch, undefined, [target[key], value]);
    } else {
      target[key] = value;
    }
  }
  return target;
}

module.exports = mergePatch;
