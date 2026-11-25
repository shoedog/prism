const config = require('config');
function run() {
  const mode = config.get('mode');
  const json = config.util.toObject();
  console.log(mode, json);
}
