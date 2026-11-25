let config = require('config');
config = { get: () => 'fixed' };
function run() {
  const mode = config.get('mode');
  console.log(mode);
}
