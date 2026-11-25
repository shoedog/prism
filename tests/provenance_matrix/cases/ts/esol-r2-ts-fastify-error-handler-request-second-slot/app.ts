import fastify from 'fastify';
const app = fastify();
app.setErrorHandler((failure, incoming, outgoing) => {
  const agent = incoming.headers['user-agent'];
  app.log.error(failure, agent);
  outgoing.code(500).send('failed');
});
