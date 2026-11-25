const copies = {
  update(_current, incoming) {
    return [...incoming];
  }
};

const reports = {
  update(target, source) {
    for (const [key, incoming] of Object.entries(source)) {
      if (Array.isArray(incoming)) {
        target[key] = copies.update(target[key], incoming);
      } else if (incoming !== null && typeof incoming === "object") {
        target[key] = reports.update(target[key] || {}, incoming);
      } else {
        target[key] = incoming;
      }
    }
    return target;
  }
};

function updateReport(source) {
  return reports.update({}, source);
}

module.exports = updateReport;
