import Fastify from 'fastify';
const app = Fastify();
app.setNotFoundHandler((incoming, outgoing) => {
  const requested = incoming.raw.url;
  outgoing.send({requested});
});
