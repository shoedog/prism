const Hapi = require('@hapi/hapi');
const app = Hapi.server();
function status() { return 'ok'; }
const plugin = {
  name: 'nested',
  async register(server) {
    function install() {
      server.route({ method: 'GET', path: '/health', handler: status });
    }
    install();
  }
};
app.register(plugin, { routes: { prefix: '/api' } });
module.exports = app;
