const Hapi = require('@hapi/hapi');
const app = Hapi.server();
function list() { return []; }
const implied = {
  name: 'features',
  register(extra) {
    extra.route({ method: 'GET', path: '/features', handler: list });
  }
};
const direct = {
  name: 'public',
  register(api) {
    api.route({ method: 'GET', path: '/public', handler: list });
  }
};
const extraPlugins = [implied];
async function build() {
  await app.register([...extraPlugins, direct]);
  return app;
}
module.exports = build;
