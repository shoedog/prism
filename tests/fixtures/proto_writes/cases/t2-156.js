function clearActive(section, field = "active") {
  const defaults = Object.create(null);
  const views = [Object.create(defaults), Object.create(defaults)];
  const draft = { active: true };
  const sections = Object.assign(Object.create(null), { draft, defaults });
  if (sections[section] !== undefined) {
    sections[section][field] &&= false;
  }
  return { views, draft };
}

module.exports = clearActive;
