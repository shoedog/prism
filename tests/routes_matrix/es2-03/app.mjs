import express from 'express';
import stock from './stock.mjs';
import changes from './changes.cjs';

const app = express();

app.get('/changes', changes);
app.get('/stock', stock);

export default app;
