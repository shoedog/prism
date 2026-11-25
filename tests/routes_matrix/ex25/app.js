const express = require('express');
const app = express();

const router = express.Router();
const sub = express.Router({ mergeParams: true });

function status(req, res) { res.sendStatus(204); }
sub.get('/status', status);
router.use('/sub', sub);
app.use('/api', router);

module.exports = app;
