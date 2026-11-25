const Hapi = require('@hapi/hapi');
const app = Hapi.server();
function health() { return 'ok'; }
const plugin = {
  name: 'health',
  register(server = app) {
    server.route({ method: 'GET', path: '/health', handler: health });
  }
};
app.register(plugin);
module.exports = app;
