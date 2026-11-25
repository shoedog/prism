import express from 'express';
const app = express();
app.set('query parser', (_raw: string) => ({background: false}));
app.get('/assets', (incoming, outgoing) => {
  const condition = incoming.headers['if-none-match'];
  const background = incoming.query.background;
  outgoing.json([condition, background]);
});
