const Hapi = require('@hapi/hapi');

const app = Hapi.server();
const container = {
    register(task) { this.task = task; },
};

container.register(require('./users'));

module.exports = app;
