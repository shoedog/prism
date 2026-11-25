function put(root: any, selector: string, property: string, value: unknown) {
  const key = selector satisfies string;
  (root as any)![key as string][property] = value;
  return root;
}

function putFromParts(root: any, [selector, property]: string[], value: unknown) {
  return put(root, selector, property, value);
}

module.exports = putFromParts;
