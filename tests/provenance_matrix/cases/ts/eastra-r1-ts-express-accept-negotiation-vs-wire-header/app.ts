import express from 'express';
const app = express();
app.get('/edition', (incoming, reply) => {
  const offered = incoming.accepts('html', 'json');
  const wire = incoming.get('Accept');
  const summary = [offered, wire];
  reply.json(summary);
});
