function annotateFormatter(section, field, value) {
  function format() {
    return "Report";
  }
  const formatter = format.bind(null);
  formatter.draft = Object.create(null);
  if (!Object.hasOwn(formatter, section)) return formatter;
  const selected = formatter[section];
  if (selected !== null && typeof selected === "object") {
    formatter[section][field] = value;
  }
  return formatter;
}

function annotateFormatterFromPath([section, field], value) {
  return annotateFormatter(section, field, value);
}

module.exports = annotateFormatterFromPath;
