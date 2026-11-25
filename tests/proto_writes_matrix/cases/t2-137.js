function prepareReportEdit(section, field) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  const selected = sections[section];
  return {
    previews,
    draft: sections.draft,
    commit(value) {
      if (selected === undefined) return false;
      selected[field] = value;
      return true;
    }
  };
}

function editFromPath([section, field], value) {
  const edit = prepareReportEdit(section, field);
  edit.commit(value);
  return edit;
}

module.exports = editFromPath;
