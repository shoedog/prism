function applyTitle(scope, publishTitle) {
  const defaults = Object.create(null);
  const cards = [Object.create(defaults), Object.create(defaults)];
  const scopes = Object.assign(Object.create(null), { all: defaults, first: cards[0] });
  const selected = scopes[scope];
  if (selected === undefined) return cards;
  const patch = {};
  Object.defineProperty(patch, "title", { value: "Preview", enumerable: publishTitle });
  Object.assign(selected, patch);
  return cards;
}

module.exports = applyTitle;
