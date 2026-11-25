function put(root: any, keys: string[], value: unknown): void;
function put(root: any, keys: readonly string[], value: unknown): void {
  const [selector, property] = keys;
  root[selector][property] = value;
}

function putFromParts(root: any, keys: string[], value: unknown) {
  put(root, keys, value);
  return root;
}

module.exports = putFromParts;
