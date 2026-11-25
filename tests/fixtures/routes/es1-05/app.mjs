import express from 'express';
import { documentation } from './index.mjs';
const app = express();
app.use('/docs', documentation);
export default app;
