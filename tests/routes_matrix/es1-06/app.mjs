import express from 'express';
import * as billing from './index.mjs';
const app = express();
if (billing.router) app.use('/billing', billing.router);
export default app;
