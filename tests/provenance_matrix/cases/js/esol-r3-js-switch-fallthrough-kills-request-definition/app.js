const express = require('express');
const app = express();
app.get('/catalog', (incoming, reply) => {
  let label;
  switch (incoming.query.preview) {
    case 'yes':
      label = incoming.query.label;
    default:
      label = process.env.PREVIEW_LABEL;
  }
  reply.send(label);
});
