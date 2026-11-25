const Hapi = require('@hapi/hapi');
const app = Hapi.server();
function live() { return 'live'; }
function draft() { return 'draft'; }

app.route({ method: 'GET', path: '/live', handler: live });

function preview(require) {
  const Hapi = require('@hapi/hapi');
  const server = Hapi.server();
  server.route({ method: 'GET', path: '/draft', handler: draft });
}

preview(() => ({ server: () => ({ route() {} }) }));
module.exports = app;
