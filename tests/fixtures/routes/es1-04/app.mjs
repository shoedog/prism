import express from 'express';
import { shipment as shipping } from './api.mjs';
const app = express();
app.use('/ops', shipping);
export default app;
