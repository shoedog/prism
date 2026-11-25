import Fastify from 'fastify';
const app = Fastify();
app.register(async scoped => {
  scoped.get<{ Querystring: { label: string } }>('/preview', async incoming => {
    const submitted = incoming.query.label;
    return submitted;
  });
}, { prefix: '/catalog' });
