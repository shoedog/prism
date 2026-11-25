function assignText(text) {
  return Object.assign({}, JSON.parse(text));
}

function assign(source) {
  return assignText(JSON.stringify(source));
}

module.exports = assign;
