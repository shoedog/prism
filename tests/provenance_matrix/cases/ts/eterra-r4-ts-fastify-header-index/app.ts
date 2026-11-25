import Fastify from 'fastify';
const app = Fastify();
app.get('/', async (request, reply) => {
  const token = request.headers['x-token'];
  return token;
});
