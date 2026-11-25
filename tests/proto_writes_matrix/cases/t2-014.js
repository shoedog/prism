class Registry {
  constructor() {
    this.items = {};
  }
  put(selector, property, value) {
    this[selector][property] = value;
  }
}

function putFromParts([selector, property], value) {
  const registry = new Registry();
  registry.put(selector, property, value);
  return registry;
}

module.exports = putFromParts;
