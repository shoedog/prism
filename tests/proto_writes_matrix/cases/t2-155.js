function prepareColumns(record, field = "enumerable") {
  const specs = Object.getOwnPropertyDescriptors(record);
  for (const name of Object.keys(specs)) {
    specs[name][field] = true;
  }
  return specs;
}

module.exports = prepareColumns;
