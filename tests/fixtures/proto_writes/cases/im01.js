function merge(root, input) {
  const stack = [[root, input]];
  while (stack.length > 0) {
    const [target, source] = stack.pop();
    for (const key in source) {
      if (source[key] && typeof source[key] === "object") {
        if (!target[key]) target[key] = {};
        stack.push([target[key], source[key]]);
      } else {
        target[key] = source[key];
      }
    }
  }
}

module.exports = merge;
