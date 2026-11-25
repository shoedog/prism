const { server } = require('@hapi/hapi');
const app = server();
function health() { return 'ok'; }
app.route({ method: 'GET', path: '/health', handler: health });
module.exports = app;
