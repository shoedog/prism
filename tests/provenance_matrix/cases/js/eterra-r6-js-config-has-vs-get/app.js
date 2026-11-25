const config = require('config');
function read() {
  const exists = config.has('port');
  const port = config.get('port');
  return [exists, port];
}
