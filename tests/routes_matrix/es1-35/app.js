const Hapi = require('@hapi/hapi');
const app = Hapi.server();
function list() { return []; }
const reports = {
  name: 'reports',
  register(reportServer) {
    reportServer.route({ method: 'GET', path: '/reports', handler: list });
  }
};
const alerts = {
  name: 'alerts',
  register(alertServer) {
    alertServer.route({ method: 'GET', path: '/alerts', handler: list });
  }
};
async function build() {
  await app.register([
    { plugin: reports, routes: { prefix: '/team' } },
    alerts
  ], { routes: { prefix: '/ops' } });
  return app;
}
module.exports = build;
