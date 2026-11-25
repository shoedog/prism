const copies = {
  update(_current, incoming) {
    return { ...incoming };
  }
};

const reports = {
  update(target, source) {
    for (const [key, incoming] of Object.entries(source)) {
      target[key] = copies.update(target[key], incoming);
    }
    return target;
  }
};

function updateReport(source) {
  return reports.update(Object.create(null), source);
}

module.exports = updateReport;
