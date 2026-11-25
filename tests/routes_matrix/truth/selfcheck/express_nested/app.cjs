const express = require('express');
const router = express.Router();
router.get('/', function root(_req, res) { res.end(); });
const child = express();
child.use(router);
const app = express();
app.use('/v1.core/', child);
app.use('/v2', child);
module.exports = { app };
