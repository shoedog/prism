function* annotateAfterSelection(choices, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  const section = yield* choices;
  if (typeof section === "string" && sections[section] !== undefined) {
    sections[section][field] = value;
  }
  return { previews, draft: sections.draft };
}

function annotateFromPath([section, field], value) {
  function* choices() {
    return section;
  }
  const run = annotateAfterSelection(choices(), field, value);
  let step = run.next();
  while (!step.done) step = run.next();
  return step.value;
}

module.exports = annotateFromPath;
