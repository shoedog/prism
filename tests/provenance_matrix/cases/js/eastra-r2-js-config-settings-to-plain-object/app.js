const settings = require('config');
function snapshot() {
  const plain = settings.util.toObject();
  const edition = plain.edition;
  const present = Object.hasOwn(plain, 'edition');
  return [edition, present];
}
