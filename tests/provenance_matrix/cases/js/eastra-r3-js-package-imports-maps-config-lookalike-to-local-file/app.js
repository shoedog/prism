const local = require('#config');
const settings = require('config');
function labels() {
  const fallback = local.get('edition');
  const deployed = settings.get('edition');
  return [fallback, deployed];
}
