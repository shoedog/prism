function define(o, v) {
  Object.defineProperty(o, '__proto__', { value: v });
}

module.exports = define;
