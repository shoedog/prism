const copies = {
  update(_current, incoming) {
    return { ...incoming };
  }
};

const reports = {
  update(target, source) {
    let writer = reports;
    writer = copies;
    for (const [key, incoming] of Object.entries(source)) {
      target[key] = writer.update(target[key], incoming);
    }
    return target;
  }
};

function updateReport(source) {
  return reports.update(Object.create(null), source);
}

module.exports = updateReport;
