function put(root, selector, property, value) {
  const holder = {
    get current() {
      return root[selector];
    }
  };
  holder.current[property] = value;
  return root;
}

function putFromParts(root, [selector, property], value) {
  return put(root, selector, property, value);
}

module.exports = putFromParts;
