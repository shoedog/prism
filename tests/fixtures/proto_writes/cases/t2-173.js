function labelGrade(section, field = "description") {
  const codes = Object.assign(Object.create(null), { pass: "P", review: "R" });
  const code = codes[section];
  if (code === undefined) return undefined;
  const patch = { [field]: "Recorded grade" };
  return Object.assign(code, patch);
}

module.exports = labelGrade;
