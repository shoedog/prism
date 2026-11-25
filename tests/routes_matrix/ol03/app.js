const { app, route } = require('./route');

route([{ method: 'GET', path: '/x', handler: (_request, response) => response.end('ready') }]);

module.exports = app;
