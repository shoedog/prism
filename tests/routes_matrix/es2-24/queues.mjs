function depth() { return { depth: 0 }; }

const queuePlugin = {
  name: 'queues',
  register(server) {
    server.route({ method: 'GET', path: '/depth', handler: depth });
  },
};

export { queuePlugin as plugin };
