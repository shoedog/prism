function makeBadge(field, label = "Ready") {
  let setterCalls = 0;
  class BaseBadge {
    set title(value) { setterCalls++; }
  }
  class Badge extends BaseBadge {
    [field] = label;
  }
  const badge = new Badge();
  return { badge, setterCalls };
}

module.exports = makeBadge;
