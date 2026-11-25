const settings = require('config');
function paths() {
  const installed = require.resolve('config');
  const configured = settings.get('output.path');
  return [installed, configured];
}
