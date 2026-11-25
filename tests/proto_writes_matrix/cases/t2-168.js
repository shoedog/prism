function openSection(path, field = "open") {
  const page = { account: { details: {} }, team: { roster: {} } };
  edit: {
    let cursor = page;
    for (const key of path.split(".")) {
      if (!Object.hasOwn(cursor, key)) break edit;
      cursor = cursor[key];
    }
    cursor[field] = true;
  }
  return page;
}

module.exports = openSection;
