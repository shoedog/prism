function status() { return 'ok'; }
module.exports = {
    name: 'inner',
    register(child) {
        child.route({ method: 'GET', path: '/health', handler: status });
    },
};
