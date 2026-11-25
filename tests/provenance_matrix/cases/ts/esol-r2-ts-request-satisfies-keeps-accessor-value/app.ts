import express from 'express';
const app = express();
app.get('/find', (incoming, reply) => {
  const phrase = incoming.query.q satisfies unknown;
  reply.send(phrase);
});
