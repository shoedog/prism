import express from 'express';
import handlers from './handlers.cjs';

const router = express.Router();
router.get('/a', handlers.second, handlers.first);
router.get('/b', handlers.second);
const app = express();
app.use('/v1', router);
export default app;
