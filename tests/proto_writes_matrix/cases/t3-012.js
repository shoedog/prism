function registerCallback(path, count) {
  const [name] = path.split(".");
  const callbacks = {};
  function callback() {
    return count;
  }
  Object.defineProperty(callbacks, name, { value: callback, enumerable: true });
  callbacks[name].argumentCount = count;
  return callbacks;
}

module.exports = registerCallback;
