const inner = require('./inner');
module.exports = {
    name: 'outer',
    async register(server) {
        await server.register(inner, { routes: { prefix: '/admin' } });
    },
};
