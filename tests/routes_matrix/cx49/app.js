const Hapi = require('@hapi/hapi');
const app = Hapi.server();

app.route({ method: 'GET', path: '/uptime', config: { handler: () => 'up' } });
module.exports = app;
