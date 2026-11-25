const Hapi = require('@hapi/hapi');
const app = Hapi.server();
function queue() { return []; }
function registerRoutes(target) {
  target.route({ method: 'GET', path: '/queue', handler: queue });
}
const plugin = {
  name: 'queues',
  register(api) {
    registerRoutes(api);
  }
};
async function build() {
  await app.register(plugin, { routes: { prefix: '/ops' } });
  return app;
}
module.exports = build;
