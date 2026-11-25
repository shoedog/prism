const settings = require('config');
function read() {
  const edition = settings.get('edition');
  return edition;
}
module.exports = read;
