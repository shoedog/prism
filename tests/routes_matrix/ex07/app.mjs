import express from 'express';
import { list } from './users.mjs';

const app = express();
app.get('/users', list);

export default app;
