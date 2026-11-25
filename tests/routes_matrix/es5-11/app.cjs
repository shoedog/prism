const Hapi = require('@hapi/hapi');
const app = Hapi.server();

function draft() { return 'draft'; }
function published() { return 'published'; }

app.route({ method: 'GET', path: '/bulletin', handler: draft, 'handler': published });

module.exports = app;
