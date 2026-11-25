class Store {
  private root: any;

  constructor(root: any) {
    this.root = root;
  }
  put(selector: string, property: string, value: unknown): void {
    this.root[selector][property] = value;
  }
}

function putFromParts(root: any, [selector, property]: string[], value: unknown) {
  const store = new Store(root);
  store.put(selector, property, value);
  return root;
}

module.exports = putFromParts;
