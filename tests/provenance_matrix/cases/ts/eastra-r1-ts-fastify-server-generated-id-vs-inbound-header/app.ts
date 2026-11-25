import Fastify from 'fastify';
const app = Fastify({ requestIdHeader: false, genReqId: () => 'local-ticket' });
app.addHook('onRequest', async incoming => {
  const generated = incoming.id;
  const supplied = incoming.headers['x-request-id'];
  void [generated, supplied];
});
app.get('/edition', async () => 'ok');
