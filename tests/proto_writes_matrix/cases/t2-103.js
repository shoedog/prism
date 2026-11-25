function setting(root, section, field, fallback) {
  return {
    get current() {
      return (root[section][field] ??= fallback);
    }
  };
}

function settingFromPath([section, field], fallback) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const root = Object.assign(Object.create(null), { draft: {}, defaults });
  const current = setting(root, section, field, fallback).current;
  return { current, previews, draft: root.draft };
}

module.exports = settingFromPath;
