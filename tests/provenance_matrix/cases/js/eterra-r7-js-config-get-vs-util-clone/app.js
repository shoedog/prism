const config = require('config');
function read() {
  const value = config.get('service');
  const copy = config.util.cloneDeep(value);
  return [value, copy];
}
