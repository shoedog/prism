function put(root, options, property, value) {
  const checked = options.key;
  if (typeof checked !== "string") return root;
  if (checked === "__proto__" || checked === "constructor") return root;
  const selector = options.key;
  if (root[selector] != null) {
    root[selector][property] = value;
  }
  return root;
}

function putFromParts(root, parts, value) {
  const [first, second, property] = parts;
  let reads = 0;
  const options = {
    get key() {
      reads++;
      return reads === 1 ? first : second;
    }
  };
  return put(root, options, property, value);
}

module.exports = putFromParts;
