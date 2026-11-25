function highlight(scope, field = "highlighted") {
  const defaults = Object.create(null);
  const cards = [Object.create(defaults), Object.create(defaults)];
  const scopes = Object.assign(Object.create(null), { all: defaults, first: cards[0] });
  function mark(selected = scope) {
    const scope = scopes[selected];
    if (scope !== undefined) scope[field] = true;
  }
  mark();
  return cards;
}

module.exports = highlight;
