function bulletins() {
  return { items: [] };
}

export async function register(server) {
  server.route({ method: 'GET', path: '/bulletins', handler: bulletins });
}
