const config = require('config');
function run() {
  const mode = config.get('mode');
  console.log(mode);
}
