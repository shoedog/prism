function annotateDisplay(section, field, value) {
  const displaySymbol = Symbol.for("display");
  const panels = { display: {}, [displaySymbol]: {} };
  const [key] = Reflect.ownKeys({ [section]: true });
  if (!Object.hasOwn(panels, key)) return panels;
  panels[key][field] = value;
  return panels;
}

function annotateDisplayAtPath(path, value) {
  const [section, field] = path;
  return annotateDisplay(section, field, value);
}

module.exports = annotateDisplayAtPath;
