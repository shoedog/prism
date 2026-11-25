const Hapi = require('@hapi/hapi');
const app = Hapi.server();

function byId() { return 'report'; }
function exportReport() { return 'export'; }
function archive() { return 'archive'; }

app.route([
  { method: 'GET', path: '/report/{id}', handler: byId },
  { method: 'GET', path: '/export/{format?}', handler: exportReport },
  { method: 'GET', path: '/archive/{parts*2}', handler: archive },
]);

module.exports = app;
