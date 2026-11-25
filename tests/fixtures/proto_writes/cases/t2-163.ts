function queueField(field: string) {
  type Sheet = { active: string; pages: Record<string, Record<string, string>> };
  function put(this: Sheet, name: string) {
    this.pages[this.active][name] = "queued";
  }
  const sheet: Sheet = { active: "inbox", pages: { inbox: Object.create(null) } };
  put.call(sheet, field);
  return sheet.pages.inbox;
}

module.exports = queueField;
