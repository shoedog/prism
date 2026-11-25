function put(selector, property, value) {
  const choices = new Map([
    ["shared", Object.prototype],
    ["local", {}]
  ]);
  const target = choices.get(selector);
  if (target) target[property] = value;
  return target;
}

function putFromParts([selector, property], value) {
  return put(selector, property, value);
}

module.exports = putFromParts;
