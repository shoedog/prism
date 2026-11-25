function readIni(out, text) {
  let section = out;
  const scratch = {};
  for (const line of text.split(/\r?\n/)) {
    const header = /^\[([^\]]+)\]$/.exec(line);
    if (header) {
      const name = header[1];
      if (name === '__proto__') {
        section = scratch;
      } else {
        section = out[name] = out[name] || {};
      }
      continue;
    }
    const separator = line.indexOf("=");
    if (separator < 0) continue;
    const key = line.slice(0, separator).trim();
    const value = line.slice(separator + 1).trim();
    section[key] = Number(value);
  }
}

module.exports = readIni;
