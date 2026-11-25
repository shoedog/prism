const { app, route } = require('./route');
const { routes } = require('./routes');

route(routes);

module.exports = app;
