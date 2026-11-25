const Hapi = require('@hapi/hapi');
const app = Hapi.server();
function health() { return 'ok'; }
const plugin = {
  name: 'health',
  register(api) {
    api.route({ method: 'GET', path: '/health', handler: health });
  }
};
async function build() {
  await app.register(plugin, { routes: { prefix: '/early' } });
  await app.register(plugin, { once: true, routes: { prefix: '/later' } });
  return app;
}
module.exports = build;
