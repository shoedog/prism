function current() { return { level: 'normal' }; }

module.exports = { method: 'GET', path: '/snapshot', handler: current };
