function annotateProfile(section, field, value) {
  const profile = {};
  const panels = new Proxy(Object.create(null), {
    get(_target, key) {
      return key === "profile" ? profile : undefined;
    }
  });
  const selected = panels[section];
  if (selected !== undefined) selected[field] = value;
  return profile;
}

module.exports = annotateProfile;
