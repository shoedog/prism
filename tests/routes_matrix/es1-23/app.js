const { server: createServer } = require('@hapi/hapi');
const app = createServer();
function ping() { return 'pong'; }

async function build() {
  await app.register({
    name: 'probe',
    register(api) {
      api.route({ method: 'GET', path: '/ping', handler: ping });
    }
  }, { routes: { prefix: '/ops' } });
  return app;
}

module.exports = build;
