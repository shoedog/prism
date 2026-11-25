import express from 'express';
import jobs from './jobs.cjs';
const app = express();
app.use('/ops', jobs);
export default app;
