import express from 'express';
import users from './users.ts';

const app = express();
app.use('/api', users);

export default app;
