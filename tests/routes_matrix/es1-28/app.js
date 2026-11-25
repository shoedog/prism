const Hapi = require('@hapi/hapi');
const app = Hapi.server();
function ready() { return 'ready'; }
const local = [{ method: 'GET', path: '/ready', handler: ready }];
const analytics = [{ method: 'GET', path: '/views', handler: ready }];
app.route(local);
app.route([{ method: 'GET', path: '/health', handler: ready }, ...analytics]);
module.exports = app;
