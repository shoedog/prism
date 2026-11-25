import express from 'express';
import { serve } from './users.mjs';

const app = express();

function makeStatusHandler() {
  function serve(req, res) { res.send('status'); }
  return serve;
}

app.get('/users', serve);

export default app;
