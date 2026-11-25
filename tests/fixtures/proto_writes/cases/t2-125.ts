const Panel: Readonly<Record<number, string | undefined>> = Object.freeze({
  0: "Profile", 1: "Display"
});

function annotatePanel(index: unknown, field: string, value: unknown) {
  const panels: Record<string, Record<string, unknown>> = {
    Profile: {}, Display: {}
  };
  if (typeof index !== "number") return panels;
  const name = Panel[index];
  const panel = name === undefined ? undefined : panels[name];
  if (panel !== undefined) panel[field] = value;
  return panels;
}

function annotatePanelFromPath([index, field]: [string, string], value: unknown) {
  return annotatePanel(Number(index), field, value);
}

module.exports = annotatePanelFromPath;
