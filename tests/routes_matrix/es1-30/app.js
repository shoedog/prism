const Hapi = require('@hapi/hapi');
const app = new Hapi.Server();
function metrics() { return 'metrics'; }
const plugin = {
  name: 'statistics',
  register(api) {
    api.route({ method: 'GET', path: '/', handler: metrics });
  }
};
async function build() {
  await app.register(plugin, { routes: { prefix: '/metrics' } });
  return app;
}
module.exports = build;
