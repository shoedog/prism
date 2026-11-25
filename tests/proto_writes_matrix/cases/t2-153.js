function markEditable(section, field = "selected") {
  class Panel {
    #editable;
    static mark(panels, key) {
      const item = panels[key];
      if (item === null || typeof item !== "object") return;
      if (#editable in item) item[field] = true;
    }
  }
  const panels = { draft: new Panel(), preview: {} };
  Panel.mark(panels, section);
  return panels;
}

module.exports = markEditable;
