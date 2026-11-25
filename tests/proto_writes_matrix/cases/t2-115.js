function annotatePanel(label, field, value) {
  const panels = {
    [Symbol.for("display")]: {},
    [Symbol.for("profile")]: {}
  };
  const key = Symbol.for(label);
  if (panels[key] == null) return panels;
  panels[key][field] = value;
  return panels;
}

module.exports = annotatePanel;
