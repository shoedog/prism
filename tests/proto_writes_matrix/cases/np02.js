function assign(o, v) {
  Object.assign(o, { ['__proto__']: v });
}

module.exports = assign;
