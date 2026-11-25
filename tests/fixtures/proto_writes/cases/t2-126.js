function annotateReport(keys, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const draft = {};
  class Editor {
    #cursor = { draft, defaults };
    apply() {
      for (const key of keys) {
        if (!Object.hasOwn(this.#cursor, key)) return;
        this.#cursor = this.#cursor[key];
      }
      this.#cursor[field] = value;
    }
  }
  new Editor().apply();
  return { previews, draft };
}

function annotateFromPath(path, value) {
  return annotateReport(path.slice(0, -1), path[path.length - 1], value);
}

module.exports = annotateFromPath;
