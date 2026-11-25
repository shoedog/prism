const Hapi = require('@hapi/hapi');
const app = Hapi.server();

function settings() { return 'settings'; }
function health() { return 'ready'; }

const plugin = {
  name: 'operations',
  register(server) {
    const api = server;
    api.route({ method: 'GET', path: '/settings', handler: settings });
    server.route({ method: 'GET', path: '/health', handler: health });
  },
};

async function configure() {
  await app.register(plugin, { routes: { prefix: '/ops' } });
  return app;
}

module.exports = { app, configure };
