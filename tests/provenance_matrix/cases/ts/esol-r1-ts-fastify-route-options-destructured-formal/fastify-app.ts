import Fastify, { type FastifyRequest } from 'fastify';

const app = Fastify();
app.route<{ Body: { title: string } }>({
  method: 'POST',
  url: '/notes',
  handler: async function ({ body: { title } }: FastifyRequest<{ Body: { title: string } }>) {
    return title;
  },
});
